use proc_macro2::Span;
use quote::ToTokens;
use ruff_python_ast::*;
use ruff_python_parser::ParseError;
use syn::parse_quote;

mod context;
pub mod input;
mod lexer;
mod lift;

use self::{
    context::context,
    input::{MacroInput, OutputKind},
    lift::{CodeFragment, Lift},
};

/// Expands a `py_quote!` invocation. Called by the proc-macro shim
/// with the raw token stream.
pub fn expand(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    match expand_inner(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn expand_inner(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let input: MacroInput = syn::parse2(input)?;

    // Preprocess: replace `#{var}` placeholders with stand-ins.
    let (preprocessed, stand_ins) = lexer::preprocess(input.source.value(), &input.variables)
        .map_err(|err| syn::Error::new(input.source.span(), err))?;

    let parsed = parse_source(preprocessed, &input.output_kind)
        .map_err(|err| syn::Error::new(input.source.span(), err))?;

    let (stmts, context) = context(&input, stand_ins)?;
    let expr = match parsed.lift(&context)? {
        CodeFragment::Single(expr) => expr,
        CodeFragment::Splice(_) => {
            return Err(syn::Error::new(
                Span::call_site(),
                "`Vec<T>` or `Option<T>` variable used in non-iterable position",
            ));
        }
    };

    let block: syn::Expr = parse_quote!({
        #(#stmts)*
        #expr
    });

    Ok(block.to_token_stream())
}

fn parse_source(source: String, kind: &OutputKind) -> Result<Box<dyn Lift>, ParseSourceError> {
    use ruff_python_parser::{parse_expression, parse_module};

    // Wrap source if needed.
    let input = match kind {
        OutputKind::Parameter | OutputKind::ParameterWithDefault => {
            format!("def __f__({source}): pass")
        }
        OutputKind::Decorator => format!("@{source}\ndef __f__(): pass"),
        OutputKind::Keyword => format!("__f__({source})"),
        OutputKind::Alias => format!("from __x__ import {source}"),
        OutputKind::Expr
        | OutputKind::Stmt
        | OutputKind::Suite
        | OutputKind::Identifier
        | OutputKind::FunctionDef
        | OutputKind::ClassDef
        | OutputKind::ImportFrom => source,
    };

    let parsed: Box<dyn Lift> = match kind {
        OutputKind::Expr => {
            let parsed = parse_expression(&input)?;
            Box::new(*parsed.into_syntax().body)
        }
        OutputKind::Identifier => {
            let parsed = parse_expression(&input)?;
            match *parsed.into_syntax().body {
                Expr::Name(name) => Box::new(Identifier {
                    id: name.id,
                    range: name.range,
                    node_index: name.node_index,
                }),
                _ => Err(Expected::Identifier)?,
            }
        }
        OutputKind::Stmt => {
            let parsed = parse_module(&input)?;
            let mut body = parsed.into_syntax().body;
            if body.is_empty() {
                Err(Expected::Statement)?;
            }
            Box::new(body.swap_remove(0))
        }
        OutputKind::Suite => {
            let parsed = parse_module(&input)?;
            let body = parsed.into_syntax().body;
            if body.is_empty() {
                Err(Expected::Statement)?;
            }
            Box::new(body)
        }
        OutputKind::FunctionDef => {
            let parsed = parse_module(&input)?;
            let mut body = parsed.into_syntax().body;
            if body.is_empty() {
                Err(Expected::Statement)?;
            }
            match body.swap_remove(0) {
                Stmt::FunctionDef(f) => Box::new(f),
                _ => Err(Expected::FunctionDef)?,
            }
        }
        OutputKind::ClassDef => {
            let parsed = parse_module(&input)?;
            let mut body = parsed.into_syntax().body;
            if body.is_empty() {
                Err(Expected::Statement)?;
            }
            match body.swap_remove(0) {
                Stmt::ClassDef(c) => Box::new(c),
                _ => Err(Expected::ClassDef)?,
            }
        }
        OutputKind::ImportFrom => {
            let parsed = parse_module(&input)?;
            let mut body = parsed.into_syntax().body;
            if body.is_empty() {
                Err(Expected::Statement)?;
            }
            match body.swap_remove(0) {
                Stmt::ImportFrom(i) => Box::new(i),
                _ => Err(Expected::ImportFrom)?,
            }
        }
        OutputKind::Parameter => {
            let parsed = parse_module(&input)?;
            let mut body = parsed.into_syntax().body;
            match body.pop() {
                Some(Stmt::FunctionDef(mut f)) => {
                    if f.parameters.args.is_empty() {
                        Err(Expected::Parameter)?;
                    }
                    Box::new(f.parameters.args.swap_remove(0).parameter)
                }
                _ => Err(Expected::FunctionDef)?,
            }
        }
        OutputKind::ParameterWithDefault => {
            let parsed = parse_module(&input)?;
            let mut body = parsed.into_syntax().body;
            match body.pop() {
                Some(Stmt::FunctionDef(mut f)) => {
                    if f.parameters.args.is_empty() {
                        Err(Expected::ParameterWithDefault)?;
                    }
                    Box::new(f.parameters.args.swap_remove(0))
                }
                _ => Err(Expected::FunctionDef)?,
            }
        }
        OutputKind::Decorator => {
            let parsed = parse_module(&input)?;
            let mut body = parsed.into_syntax().body;
            match body.pop() {
                Some(Stmt::FunctionDef(mut f)) => {
                    if f.decorator_list.is_empty() {
                        Err(Expected::Decorator)?;
                    }
                    Box::new(f.decorator_list.swap_remove(0))
                }
                _ => Err(Expected::FunctionDef)?,
            }
        }
        OutputKind::Keyword => {
            let parsed = parse_expression(&input)?;
            match *parsed.into_syntax().body {
                Expr::Call(call) => {
                    let mut keywords = call.arguments.keywords.into_vec();
                    if keywords.is_empty() {
                        Err(Expected::Keyword)?;
                    }
                    Box::new(keywords.swap_remove(0))
                }
                _ => Err(Expected::Call)?,
            }
        }
        OutputKind::Alias => {
            let parsed = parse_module(&input)?;
            let mut body = parsed.into_syntax().body;
            match body.pop() {
                Some(Stmt::ImportFrom(mut imp)) => {
                    if imp.names.is_empty() {
                        Err(Expected::Alias)?;
                    }
                    Box::new(imp.names.swap_remove(0))
                }
                _ => Err(Expected::ImportFrom)?,
            }
        }
    };

    Ok(parsed)
}

#[derive(Debug, thiserror::Error)]
enum ParseSourceError {
    #[error("failed to parse Python: {0}")]
    Parser(#[from] ParseError),
    #[error(transparent)]
    Expected(#[from] Expected),
}

#[derive(Debug, thiserror::Error)]
enum Expected {
    #[error("expected a function definition")]
    FunctionDef,
    #[error("expected a class definition")]
    ClassDef,
    #[error("expected a `from ... import ...` statement")]
    ImportFrom,
    #[error("expected at least one statement")]
    Statement,
    #[error("expected at least one parameter")]
    Parameter,
    #[error("expected at least one parameter with default")]
    ParameterWithDefault,
    #[error("expected at least one decorator")]
    Decorator,
    #[error("expected at least one keyword argument")]
    Keyword,
    #[error("expected a call expression")]
    Call,
    #[error("expected at least one alias")]
    Alias,
    #[error("expected an identifier")]
    Identifier,
}
