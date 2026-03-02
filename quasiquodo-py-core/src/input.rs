use std::fmt::Display;

use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use unindent::unindent;

/// Parsed macro input.
///
/// The macro understands these forms:
///
/// ```text
/// root; "source" as OutputKind, var: Type = expr, vars...
///
/// root; {"
///     source
/// "} as OutputKind, var: Type = expr, vars...
/// ```
///
/// `root` is the resolved crate path (e.g., `::quasiquodo_py`),
/// injected by the declarative macro wrapper. `...vars` parses
/// zero or more [`Variable`]s, declared as `name: type = value`.
pub struct MacroInput {
    pub root: syn::Path,
    pub source: syn::LitStr,
    pub output_kind: OutputKind,
    pub variables: Vec<Variable>,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let root = input.parse()?;
        input.parse::<Token![;]>()?;

        let source = if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            let lit: syn::LitStr = content.parse()?;
            syn::LitStr::new(&unindent(&lit.value()), lit.span())
        } else {
            input.parse()?
        };
        input.parse::<Token![as]>()?;
        let output_kind = input.parse()?;

        let variables = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Punctuated::<Variable, Token![,]>::parse_terminated(input)?
                .into_iter()
                .collect()
        } else {
            vec![]
        };

        Ok(Self {
            root,
            source,
            output_kind,
            variables,
        })
    }
}

/// The [`ruff_python_ast`] node type to parse from the input source.
pub enum OutputKind {
    /// Parses an [`Expr`][ruff_python_ast::Expr] using
    /// [`parse_expression`][ruff_python_parser::parse_expression].
    Expr,

    /// Extracts a [`Stmt`][ruff_python_ast::Stmt] from a module
    /// parsed with [`parse_module`][ruff_python_parser::parse_module].
    Stmt,

    /// Parses a [`Suite`][ruff_python_ast::Suite]
    /// from a module parsed with
    /// [`parse_module`][ruff_python_parser::parse_module].
    Suite,

    /// Extracts an [`Identifier`][ruff_python_ast::Identifier] from a
    /// name expression parsed with [`parse_expression`][ruff_python_parser::parse_expression].
    Identifier,

    /// Parses a [`Parameter`][ruff_python_ast::Parameter] from
    /// `def __f__(<source>): pass`.
    Parameter,

    /// Parses a [`ParameterWithDefault`][ruff_python_ast::ParameterWithDefault]
    /// from `def __f__(<source>): pass`.
    ParameterWithDefault,

    /// Parses a [`Decorator`][ruff_python_ast::Decorator] from
    /// `@<source> \n def __f__(): pass`.
    Decorator,

    /// Parses a [`Keyword`][ruff_python_ast::Keyword] from
    /// `__f__(<source>)`.
    Keyword,

    /// Parses an [`Alias`][ruff_python_ast::Alias] from
    /// `from __x__ import <source>`.
    Alias,

    /// Extracts a [`StmtFunctionDef`][ruff_python_ast::StmtFunctionDef] from
    /// a module parsed with [`parse_module`][ruff_python_parser::parse_module].
    FunctionDef,

    /// Extracts a [`StmtClassDef`][ruff_python_ast::StmtClassDef] from
    /// a module parsed with [`parse_module`][ruff_python_parser::parse_module].
    ClassDef,

    /// Extracts a [`StmtImportFrom`][ruff_python_ast::StmtImportFrom] from
    /// a module parsed with [`parse_module`][ruff_python_parser::parse_module].
    ImportFrom,
}

impl Parse for OutputKind {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "Alias" => Ok(Self::Alias),
            "ClassDef" => Ok(Self::ClassDef),
            "Decorator" => Ok(Self::Decorator),
            "Expr" => Ok(Self::Expr),
            "FunctionDef" => Ok(Self::FunctionDef),
            "Identifier" => Ok(Self::Identifier),
            "ImportFrom" => Ok(Self::ImportFrom),
            "Keyword" => Ok(Self::Keyword),
            "Parameter" => Ok(Self::Parameter),
            "ParameterWithDefault" => Ok(Self::ParameterWithDefault),
            "Stmt" => Ok(Self::Stmt),
            "Suite" => Ok(Self::Suite),
            other => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unsupported output kind `{other}`; expected one of \
                     `Alias`, `ClassDef`, `Decorator`, `Expr`, \
                     `FunctionDef`, `Identifier`, `ImportFrom`, \
                     `Keyword`, `Parameter`, `ParameterWithDefault`, \
                     `Stmt`, `Suite`"
                ),
            )),
        }
    }
}

/// A single variable binding: `name: VarType = expr`.
pub struct Variable {
    pub name: Ident,
    pub ty: VarType,
    pub value: syn::Expr,
}

impl Parse for Variable {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Variable { name, ty, value })
    }
}

/// The type of a substitution variable.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum VarType {
    /// Substitutes an `Alias` in an import statement.
    Alias,
    /// Substitutes a [`bool`] value as a `BooleanLiteral`.
    Bool,
    /// Substitutes a `Decorator` in decorator position.
    Decorator,
    /// Substitutes an `Expr` in expression position.
    Expr,
    /// Substitutes an `Identifier` in identifier position.
    Identifier,
    /// Substitutes a `Keyword` in a call's keyword arguments.
    Keyword,
    /// Substitutes a numeric value as a `NumberLiteral`.
    Num(NumVarType),
    /// Substitutes a `Parameter` in a function parameter list.
    Parameter,
    /// Substitutes a `ParameterWithDefault` in a function parameter
    /// list.
    ParameterWithDefault,
    /// Substitutes a `Stmt` in a block statement body.
    Stmt,
    /// Substitutes a `Suite` in a block statement body.
    Suite,
    /// Substitutes a string value as a `StringLiteral`.
    Str(StrVarType),

    /// Wraps an inner type in a borrowed reference.
    Ref(Box<VarType>),
    /// Wraps an inner type in `Box<Inner>`.
    Box(Box<VarType>),
    /// Wraps an inner type in `Option<Inner>`.
    Option(Box<VarType>),
    /// Splices a `Vec<Inner>` into an iterable position.
    Vec(Box<VarType>),
}

impl VarType {
    /// If this is a wrapper type, returns the wrapped type; otherwise,
    /// returns `self`.
    #[inline]
    pub fn inner(&self) -> &VarType {
        match self {
            Self::Ref(ty) | Self::Box(ty) | Self::Vec(ty) | Self::Option(ty) => ty,
            other => other,
        }
    }

    /// If this is a pointer type, returns the pointed-to type; otherwise,
    /// returns `self`.
    #[inline]
    pub fn pointee(&self) -> &VarType {
        match self {
            Self::Ref(ty) | Self::Box(ty) => ty,
            other => other,
        }
    }

    /// Returns `true` if this is a string-like type (`String`, `&str`,
    /// or `Box<str>`).
    #[inline]
    pub fn is_str(&self) -> bool {
        matches!(self, VarType::Str(StrVarType::String))
            || matches!(self.pointee(), VarType::Str(StrVarType::Str))
    }
}

impl Display for VarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alias => f.write_str("Alias"),
            Self::Bool => f.write_str("bool"),
            Self::Decorator => f.write_str("Decorator"),
            Self::Expr => f.write_str("Expr"),
            Self::Identifier => f.write_str("Identifier"),
            Self::Keyword => f.write_str("Keyword"),
            Self::Num(NumVarType::F64) => f.write_str("f64"),
            Self::Num(NumVarType::U8) => f.write_str("u8"),
            Self::Num(NumVarType::U16) => f.write_str("u16"),
            Self::Num(NumVarType::U32) => f.write_str("u32"),
            Self::Num(NumVarType::U64) => f.write_str("u64"),
            Self::Parameter => f.write_str("Parameter"),
            Self::ParameterWithDefault => f.write_str("ParameterWithDefault"),
            Self::Stmt => f.write_str("Stmt"),
            Self::Suite => f.write_str("Suite"),
            Self::Str(StrVarType::Str) => f.write_str("str"),
            Self::Str(StrVarType::String) => f.write_str("String"),
            Self::Ref(inner) => write!(f, "&{inner}"),
            Self::Box(inner) => write!(f, "Box<{inner}>"),
            Self::Option(inner) => write!(f, "Option<{inner}>"),
            Self::Vec(inner) => write!(f, "Vec<{inner}>"),
        }
    }
}

impl Parse for VarType {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let (token, span) = if input.peek(Token![&]) {
            let and = input.parse::<Token![&]>()?;
            ("&".to_owned(), and.span)
        } else {
            let ident: Ident = input.parse()?;
            (ident.to_string(), ident.span())
        };
        match &*token {
            "Alias" => Ok(Self::Alias),
            "bool" => Ok(Self::Bool),
            "Decorator" => Ok(Self::Decorator),
            "Expr" => Ok(Self::Expr),
            "f64" => Ok(Self::Num(NumVarType::F64)),
            "Identifier" => Ok(Self::Identifier),
            "Keyword" => Ok(Self::Keyword),
            "Parameter" => Ok(Self::Parameter),
            "ParameterWithDefault" => Ok(Self::ParameterWithDefault),
            "Stmt" => Ok(Self::Stmt),
            "Suite" => Ok(Self::Suite),
            "str" => Ok(Self::Str(StrVarType::Str)),
            "String" => Ok(Self::Str(StrVarType::String)),
            "u8" => Ok(Self::Num(NumVarType::U8)),
            "u16" => Ok(Self::Num(NumVarType::U16)),
            "u32" => Ok(Self::Num(NumVarType::U32)),
            "u64" => Ok(Self::Num(NumVarType::U64)),
            "&" => Ok(Self::Ref(Box::new(input.parse()?))),
            "Box" => {
                input.parse::<Token![<]>()?;
                let inner: VarType = input.parse()?;
                input.parse::<Token![>]>()?;
                Ok(Self::Box(Box::new(inner)))
            }
            "Option" => {
                input.parse::<Token![<]>()?;
                let inner: VarType = input.parse()?;
                input.parse::<Token![>]>()?;
                Ok(Self::Option(Box::new(inner)))
            }
            "Vec" => {
                input.parse::<Token![<]>()?;
                let inner: VarType = input.parse()?;
                input.parse::<Token![>]>()?;
                Ok(Self::Vec(Box::new(inner)))
            }
            other => Err(syn::Error::new(
                span,
                MacroInputError::UnsupportedVar(other),
            )),
        }
    }
}

/// The concrete Rust type for a [`VarType::Num`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NumVarType {
    F64,
    U8,
    U16,
    U32,
    U64,
}

/// The concrete Rust type for a [`VarType::Str`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StrVarType {
    Str,
    String,
}

#[derive(Debug, thiserror::Error)]
enum MacroInputError<'a> {
    #[error(
        "unsupported variable type `{0}`; expected one of \
         `Alias`, `bool`, `Decorator`, `Expr`, \
         `f64`, `Identifier`, `Keyword`, `Parameter`, \
         `ParameterWithDefault`, `Stmt`, `Suite`, `str`, \
         `String`, `u8`, `u16`, `u32`, `u64`, `&...`, \
         `Box<...>`, `Option<...>`, `Vec<...>`"
    )]
    UnsupportedVar(&'a str),
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use syn::parse_str;

    #[test]
    fn test_parse_simple() {
        let input: MacroInput = parse_str(r#"::quasiquodo_py; "x + 1" as Expr"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::Expr));
        assert!(input.variables.is_empty());
    }

    #[test]
    fn test_parse_with_variables() {
        let input: MacroInput =
            parse_str(r##"::quasiquodo_py; "#{name}()" as Expr, name: Identifier = my_name"##)
                .unwrap();
        assert!(matches!(input.output_kind, OutputKind::Expr));
        assert_eq!(input.variables.len(), 1);
        assert_eq!(input.variables[0].name, "name");
        assert!(matches!(input.variables[0].ty, VarType::Identifier));
    }

    #[test]
    fn test_parse_trailing_comma() {
        let input: MacroInput = parse_str(r#"::quasiquodo_py; "x + 1" as Expr,"#).unwrap();
        assert!(input.variables.is_empty());
    }

    #[test]
    fn test_parse_unknown_output_kind() {
        let result: syn::Result<MacroInput> = parse_str(r#"::quasiquodo_py; "x" as Bogus"#);
        assert!(result.is_err());
        let msg = result.err().expect("expected error").to_string();
        assert!(msg.contains("unsupported output kind"));
    }

    #[test]
    fn test_parse_var_type_expr() {
        let vt: VarType = parse_str("Expr").unwrap();
        assert!(matches!(vt, VarType::Expr));
    }

    #[test]
    fn test_parse_var_type_vec_expr() {
        let vt: VarType = parse_str("Vec<Expr>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::Expr)
        ));
    }

    #[test]
    fn test_parse_var_type_option_stmt() {
        let vt: VarType = parse_str("Option<Stmt>").unwrap();
        assert!(matches!(
            vt,
            VarType::Option(ref inner) if matches!(**inner, VarType::Stmt)
        ));
    }

    #[test]
    fn test_parse_var_type_identifier() {
        let vt: VarType = parse_str("Identifier").unwrap();
        assert!(matches!(vt, VarType::Identifier));
    }

    #[test]
    fn test_parse_var_type_stmt() {
        let vt: VarType = parse_str("Stmt").unwrap();
        assert!(matches!(vt, VarType::Stmt));
    }

    #[test]
    fn test_parse_var_type_vec_stmt() {
        let vt: VarType = parse_str("Vec<Stmt>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::Stmt)
        ));
    }

    #[test]
    fn test_parse_var_type_parameter() {
        let vt: VarType = parse_str("Parameter").unwrap();
        assert!(matches!(vt, VarType::Parameter));
    }

    #[test]
    fn test_parse_var_type_decorator() {
        let vt: VarType = parse_str("Decorator").unwrap();
        assert!(matches!(vt, VarType::Decorator));
    }

    #[test]
    fn test_parse_output_kind_function_def() {
        let input: MacroInput =
            parse_str(r#"::quasiquodo_py; "def foo(): pass" as FunctionDef"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::FunctionDef));
    }

    #[test]
    fn test_parse_output_kind_alias() {
        let input: MacroInput = parse_str(r#"::quasiquodo_py; "Foo" as Alias"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::Alias));
    }

    #[test]
    fn test_parse_var_type_bool() {
        let vt: VarType = parse_str("bool").unwrap();
        assert!(matches!(vt, VarType::Bool));
    }

    #[test]
    fn test_parse_var_type_f64() {
        let vt: VarType = parse_str("f64").unwrap();
        assert!(matches!(vt, VarType::Num(NumVarType::F64)));
    }

    #[test]
    fn test_parse_var_type_str() {
        let vt: VarType = parse_str("&str").unwrap();
        assert!(matches!(
            vt,
            VarType::Ref(ref inner) if matches!(**inner, VarType::Str(StrVarType::Str))));
    }

    #[test]
    fn test_parse_var_type_string() {
        let vt: VarType = parse_str("String").unwrap();
        assert!(matches!(vt, VarType::Str(StrVarType::String)));
    }

    #[test]
    fn test_parse_var_type_option_str() {
        let vt: VarType = parse_str("Option<&str>").unwrap();
        assert!(matches!(
            vt,
            VarType::Option(ref inner) if matches!(**inner, VarType::Ref(ref inner)
                if matches!(**inner, VarType::Str(StrVarType::Str)))
        ));
    }

    #[test]
    fn test_parse_output_kind_suite() {
        let input: MacroInput = parse_str(r#"::quasiquodo_py; "x = 1\ny = 2" as Suite"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::Suite));
    }

    #[test]
    fn test_parse_var_type_suite() {
        let vt: VarType = parse_str("Suite").unwrap();
        assert!(matches!(vt, VarType::Suite));
    }

    // MARK: Braced string syntax

    #[test]
    fn test_parse_braced_string() {
        let input: MacroInput = parse_str(
            r#"::quasiquodo_py; {"
            x = 1
            y = 2
        "} as Suite"#,
        )
        .unwrap();
        assert!(matches!(input.output_kind, OutputKind::Suite));
        assert_eq!(
            input.source.value(),
            indoc! {"
                x = 1
                y = 2
            "},
        );
    }

    #[test]
    fn test_parse_braced_string_preserves_relative_indent() {
        let input: MacroInput = parse_str(
            r#"::quasiquodo_py; {"
            def foo():
                pass
        "} as Stmt"#,
        )
        .unwrap();
        assert_eq!(
            input.source.value(),
            indoc! {"
                def foo():
                    pass
            "},
        );
    }

    #[test]
    fn test_parse_braced_string_with_variables() {
        let input: MacroInput = parse_str(
            r##"::quasiquodo_py; {"
            #{name}()
        "} as Expr, name: Identifier = my_name"##,
        )
        .unwrap();
        assert!(matches!(input.output_kind, OutputKind::Expr));
        assert_eq!(
            input.source.value(),
            indoc! {"
                #{name}()
            "},
        );
        assert_eq!(input.variables.len(), 1);
    }
}
