mod context;
pub mod input;
mod lexer;
mod lift;

use proc_macro2::Span;
use quote::ToTokens;
use swc_common::comments::SingleThreadedComments;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_parser::{Lexer, Parser, StringInput, Syntax, TsSyntax, error::Error as ParserError};
use syn::parse_quote;

use self::{
    context::context,
    input::{MacroInput, OutputKind},
    lift::{CodeFragment, Lift},
};

#[cfg(test)]
mod tests;

/// Expands a `ts_quote!` invocation. Called by the proc-macro shim
/// with the raw token stream.
pub fn expand(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    match expand_inner(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn expand_inner(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let input: MacroInput = syn::parse2(input)?;

    // Preprocess: replace `#{var}` placeholders with
    // type-appropriate stand-ins.
    let (preprocessed, stand_ins) = lexer::preprocess(input.source.value(), &input.variables)
        .map_err(|err| syn::Error::new(input.source.span(), err))?;

    let (parsed, comments) = parse_source(preprocessed, &input.output_kind)
        .map_err(|err| syn::Error::new(input.source.span(), err))?;

    let (stmts, context) = context(&input, stand_ins, comments)?;
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

fn parse_source(
    source: String,
    kind: &OutputKind,
) -> Result<(Box<dyn Lift>, SingleThreadedComments), ParseSourceError> {
    use swc_ecma_ast::*;

    // Wrap source if needed.
    let input = match kind {
        OutputKind::TsType => format!("type T = {source};"),
        OutputKind::ClassMember => format!("class C {{ {source} }}"),
        OutputKind::TsTypeElement => format!("interface I {{ {source} }}"),
        OutputKind::ParamOrTsParamProp => {
            format!("class C {{ constructor({source}) {{}} }}")
        }
        OutputKind::Param => format!("function f({source}) {{}}"),
        OutputKind::ImportSpecifier => format!(r#"import {{{source} }} from "";"#),
        OutputKind::ExportSpecifier => format!(r#"export {{{source} }} from "";"#),
        OutputKind::Ident
        | OutputKind::Expr
        | OutputKind::Stmt
        | OutputKind::ModuleItem
        | OutputKind::Decl => source,
    };

    let source_map = Lrc::new(SourceMap::default());
    let source_file = source_map.new_source_file(FileName::Anon.into(), input);
    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax::default()),
        EsVersion::latest(),
        StringInput::from(&*source_file),
        Some(&comments),
    );
    let mut parser = Parser::new_from(lexer);

    let parsed: Box<dyn Lift> = match kind {
        OutputKind::TsType => match parser.parse_module_item()? {
            ModuleItem::Stmt(Stmt::Decl(Decl::TsTypeAlias(alias))) => Box::new(*alias.type_ann),
            _ => Err(Expected::TypeAlias)?,
        },
        OutputKind::ClassMember => match parser.parse_module_item()? {
            ModuleItem::Stmt(Stmt::Decl(Decl::Class(class_decl))) => {
                let mut body = class_decl.class.body;
                if body.len() != 1 {
                    Err(Expected::ClassMember)?;
                }
                Box::new(body.swap_remove(0))
            }
            _ => Err(Expected::Class)?,
        },
        OutputKind::TsTypeElement => match parser.parse_module_item()? {
            ModuleItem::Stmt(Stmt::Decl(Decl::TsInterface(iface))) => {
                let mut body = iface.body.body;
                if body.len() != 1 {
                    Err(Expected::TypeElement)?;
                }
                Box::new(body.swap_remove(0))
            }
            _ => Err(Expected::Interface)?,
        },
        OutputKind::ParamOrTsParamProp => match parser.parse_module_item()? {
            ModuleItem::Stmt(Stmt::Decl(Decl::Class(class_decl))) => {
                let mut body = class_decl.class.body;
                let Some(ClassMember::Constructor(mut ctor)) = body.pop() else {
                    Err(Expected::Constructor)?
                };
                if ctor.params.len() != 1 {
                    Err(Expected::ConstructorParam)?;
                }
                Box::new(ctor.params.swap_remove(0))
            }
            _ => Err(Expected::Class)?,
        },
        OutputKind::Param => match parser.parse_module_item()? {
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl))) => {
                let mut params = fn_decl.function.params;
                if params.len() != 1 {
                    Err(Expected::Param)?;
                }
                Box::new(params.swap_remove(0))
            }
            _ => Err(Expected::Function)?,
        },
        OutputKind::ImportSpecifier => match parser.parse_module_item()? {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                let mut specifiers = import.specifiers;
                if specifiers.len() != 1 {
                    Err(Expected::ImportSpecifier)?;
                }
                Box::new(specifiers.swap_remove(0))
            }
            _ => Err(Expected::Import)?,
        },
        OutputKind::ExportSpecifier => match parser.parse_module_item()? {
            ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) => {
                let mut specifiers = export.specifiers;
                if specifiers.len() != 1 {
                    Err(Expected::ExportSpecifier)?;
                }
                Box::new(specifiers.swap_remove(0))
            }
            _ => Err(Expected::NamedExport)?,
        },
        OutputKind::Ident => match *parser.parse_expr()? {
            Expr::Ident(ident) => Box::new(ident),
            _ => Err(Expected::Identifier)?,
        },
        OutputKind::Expr => Box::new(*parser.parse_expr()?),
        OutputKind::Stmt => {
            // `parse_stmt_list_item` includes declarations
            // (`const`, `let`, `class`, etc.), unlike `parse_stmt`.
            Box::new(parser.parse_stmt_list_item()?)
        }
        OutputKind::ModuleItem => Box::new(parser.parse_module_item()?),
        OutputKind::Decl => match parser.parse_module_item()? {
            ModuleItem::Stmt(Stmt::Decl(decl)) => Box::new(decl),
            _ => Err(Expected::Declaration)?,
        },
    };

    Ok((parsed, comments))
}

#[derive(Debug, thiserror::Error)]
enum ParseSourceError {
    #[error("failed to parse TypeScript: {0:?}")]
    Parser(ParserError),
    #[error(transparent)]
    Expected(#[from] Expected),
}

impl From<ParserError> for ParseSourceError {
    fn from(err: ParserError) -> Self {
        Self::Parser(err)
    }
}

#[derive(Debug, thiserror::Error)]
enum Expected {
    #[error("expected a class declaration")]
    Class,
    #[error("expected exactly one class member")]
    ClassMember,
    #[error("expected exactly one type element")]
    TypeElement,
    #[error("expected a function declaration")]
    Function,
    #[error("expected exactly one param")]
    Param,
    #[error("expected a constructor")]
    Constructor,
    #[error("expected exactly one constructor param")]
    ConstructorParam,
    #[error("expected an import declaration")]
    Import,
    #[error("expected exactly one import specifier")]
    ImportSpecifier,
    #[error("expected a named export declaration")]
    NamedExport,
    #[error("expected exactly one export specifier")]
    ExportSpecifier,
    #[error("expected a type alias declaration")]
    TypeAlias,
    #[error("expected a declaration")]
    Declaration,
    #[error("expected an identifier")]
    Identifier,
    #[error("expected an interface declaration")]
    Interface,
}
