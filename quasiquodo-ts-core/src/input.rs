use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Token};

mod kw {
    syn::custom_keyword!(span);
    syn::custom_keyword!(comments);
}

/// Parsed macro input.
///
/// The macro understands these forms:
///
/// ```text
/// "source" as OutputKind, var: Type = expr, ...
/// span = expr, "source" as OutputKind, ...
/// span, "source" as OutputKind, ...
/// comments = expr, "source" as OutputKind, ...
/// comments, "source" as OutputKind, ...
/// ```
///
/// `...` parses zero or more [`Variable`]s.
pub struct MacroInput {
    pub span: Option<syn::Expr>,
    pub comments: Option<syn::Expr>,
    pub source: syn::LitStr,
    pub output_kind: OutputKind,
    pub variables: Vec<Variable>,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // Parse optional `span` and `comments` arguments, in either order.
        let mut span: Option<syn::Expr> = None;
        let mut comments: Option<syn::Expr> = None;
        loop {
            if input.peek(kw::span) {
                let kw = input.parse::<kw::span>()?;
                span = Some(if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    input.parse()?
                } else {
                    // A bare `span` argument means that we should use
                    // the `span` variable in scope.
                    let ident = syn::Ident::new("span", kw.span);
                    syn::parse_quote!(#ident)
                });
                input.parse::<Token![,]>()?;
            } else if input.peek(kw::comments) {
                let kw = input.parse::<kw::comments>()?;
                comments = Some(if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    input.parse()?
                } else {
                    let ident = syn::Ident::new("comments", kw.span);
                    syn::parse_quote!(#ident)
                });
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        let source = input.parse()?;
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
            span,
            comments,
            source,
            output_kind,
            variables,
        })
    }
}

/// The [`swc_ecma_ast`] node type to parse from the input source.
pub enum OutputKind {
    /// Parses a [`ClassMember`][swc_ecma_ast::ClassMember] from
    /// `class C { <source> }`.
    ClassMember,

    /// Extracts a [`Decl`][swc_ecma_ast::Decl] from a declaration statement
    /// parsed with [`parse_module_item`][swc_ecma_parser::Parser::parse_module_item].
    Decl,

    /// Parses an [`Expr`][swc_ecma_ast::Expr] using
    /// [`parse_expr`][swc_ecma_parser::Parser::parse_expr].
    Expr,

    /// Parses an [`ExportSpecifier`][swc_ecma_ast::ExportSpecifier];
    /// from `export { <source> } from "";`.
    ExportSpecifier,

    /// Extracts an [`Ident`][swc_ecma_ast::Ident] from an identifier expression
    /// parsed with [`parse_expr`][swc_ecma_parser::Parser::parse_expr].
    Ident,

    /// Parses an [`ImportSpecifier`][swc_ecma_ast::ImportSpecifier]
    /// from `import { <source> } from "";`.
    ImportSpecifier,

    /// Parses a [`ModuleItem`][swc_ecma_ast::ModuleItem] using
    /// [`parse_module_item`][swc_ecma_parser::Parser::parse_module_item].
    ModuleItem,

    /// Parses a [`Param`][swc_ecma_ast::Param] from
    /// `function f(<source>) {}`.
    Param,

    /// Parses a [`ParamOrTsParamProp`][swc_ecma_ast::ParamOrTsParamProp]
    /// from `class C { constructor(<source>) {} }`.
    ParamOrTsParamProp,

    /// Parses a [`Stmt`][swc_ecma_ast::Stmt] using
    /// [`parse_stmt_list_item`][swc_ecma_parser::Parser::parse_stmt_list_item].
    Stmt,

    /// Parses a [`TsType`][swc_ecma_ast::TsType] from
    /// `type T = <source>;`.
    TsType,

    /// Parses a [`TsTypeElement`][swc_ecma_ast::TsTypeElement] from
    /// `interface I { <source> }`.
    TsTypeElement,
}

impl Parse for OutputKind {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "ClassMember" => Ok(Self::ClassMember),
            "Decl" => Ok(Self::Decl),
            "Expr" => Ok(Self::Expr),
            "ExportSpecifier" => Ok(Self::ExportSpecifier),
            "Ident" => Ok(Self::Ident),
            "ImportSpecifier" => Ok(Self::ImportSpecifier),
            "ModuleItem" => Ok(Self::ModuleItem),
            "Param" => Ok(Self::Param),
            "ParamOrTsParamProp" => Ok(Self::ParamOrTsParamProp),
            "Stmt" => Ok(Self::Stmt),
            "TsType" => Ok(Self::TsType),
            "TsTypeElement" => Ok(Self::TsTypeElement),
            other => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unsupported output kind `{other}`; expected one of \
                     `ClassMember`, `Decl`, `Expr`, `ExportSpecifier`, \
                     `Ident`, `ImportSpecifier`, `ModuleItem`, `Param`, \
                     `ParamOrTsParamProp`, `Stmt`, `TsType`, `TsTypeElement`"
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
    /// Substitutes a `ClassMember` in a class body.
    ClassMember,
    /// Substitutes a `Decl` in declaration/statement position.
    Decl,
    /// Substitutes an `Expr` in expression position.
    Expr,
    /// Substitutes an `ExportSpecifier` in an export declaration.
    ExportSpecifier,
    /// Substitutes an `Ident` in identifier position.
    Ident,
    /// Substitutes an `ImportSpecifier` in an import declaration.
    ImportSpecifier,
    /// Substitutes a JSDoc comment, attaching it as a leading comment
    /// on the next node.
    JsDoc,
    /// Substitutes a [`bool`] value as a `TsLit::Bool` literal.
    LitBool,
    /// Substitutes an [`f64`] value as a `TsLit::Number` literal.
    LitNum,
    /// Substitutes a string slice value as a `TsLit::Str` literal.
    LitStr,
    /// Substitutes a `Param` in a function parameter list.
    Param,
    /// Substitutes a `ParamOrTsParamProp` in a constructor parameter
    /// list.
    ParamOrTsParamProp,
    /// Substitutes a `Stmt` in a block statement body.
    Stmt,
    /// Substitutes a `TsType` in type position.
    TsType,
    /// Substitutes a `TsTypeElement` in an interface or type-literal body.
    TsTypeElement,

    /// Wraps an inner type in `Box<Inner>`.
    Box(Box<VarType>),
    /// Wraps an inner type in `Option<Inner>`.
    Option(Box<VarType>),
    /// Splices a `Vec<Inner>` into an iterable position: union,
    /// intersection, interface body, extends clause, class body,
    /// parameter list, or block statement.
    Vec(Box<VarType>),
}

impl VarType {
    #[inline]
    pub fn inner(&self) -> &VarType {
        match self {
            Self::Box(ty) | Self::Vec(ty) | Self::Option(ty) => ty.inner(),
            other => other,
        }
    }

    #[inline]
    pub fn to_tokens(&self) -> VarTypeToTokens<'_> {
        VarTypeToTokens(self)
    }
}

impl Parse for VarType {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "ClassMember" => Ok(Self::ClassMember),
            "Decl" => Ok(Self::Decl),
            "Expr" => Ok(Self::Expr),
            "ExportSpecifier" => Ok(Self::ExportSpecifier),
            "Ident" => Ok(Self::Ident),
            "ImportSpecifier" => Ok(Self::ImportSpecifier),
            "JsDoc" => Ok(Self::JsDoc),
            "LitBool" => Ok(Self::LitBool),
            "LitNum" => Ok(Self::LitNum),
            "LitStr" => Ok(Self::LitStr),
            "Param" => Ok(Self::Param),
            "ParamOrTsParamProp" => Ok(Self::ParamOrTsParamProp),
            "Stmt" => Ok(Self::Stmt),
            "TsType" => Ok(Self::TsType),
            "TsTypeElement" => Ok(Self::TsTypeElement),
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
                ident.span(),
                format!(
                    "unsupported variable type `{other}`; expected one of \
                     `ClassMember`, `Decl`, `Expr`, `ExportSpecifier`, \
                     `Ident`, `ImportSpecifier`, `JsDoc`, `LitBool`, \
                     `LitNum`, `LitStr`, `Param`, `ParamOrTsParamProp`, \
                     `Stmt`, `TsType`, `TsTypeElement`, `Box<...>`, \
                     `Option<...>`, `Vec<...>`"
                ),
            )),
        }
    }
}

pub struct VarTypeToTokens<'a>(&'a VarType);

impl ToTokens for VarTypeToTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(match self.0 {
            VarType::ClassMember => quote!(::quasiquodo::ts::swc::ecma_ast::ClassMember),
            VarType::Decl => quote!(::quasiquodo::ts::swc::ecma_ast::Decl),
            VarType::Expr => quote!(::quasiquodo::ts::swc::ecma_ast::Expr),
            VarType::ExportSpecifier => {
                quote!(::quasiquodo::ts::swc::ecma_ast::ExportSpecifier)
            }
            VarType::Ident => quote!(::quasiquodo::ts::swc::ecma_ast::Ident),
            VarType::ImportSpecifier => {
                quote!(::quasiquodo::ts::swc::ecma_ast::ImportSpecifier)
            }
            VarType::JsDoc => quote!(::quasiquodo::ts::JsDoc),
            VarType::LitBool => quote!(bool),
            VarType::LitNum => quote!(f64),
            VarType::LitStr => quote!(&str),
            VarType::Param => quote!(::quasiquodo::ts::swc::ecma_ast::Param),
            VarType::ParamOrTsParamProp => {
                quote!(::quasiquodo::ts::swc::ecma_ast::ParamOrTsParamProp)
            }
            VarType::Stmt => quote!(::quasiquodo::ts::swc::ecma_ast::Stmt),
            VarType::TsType => quote!(::quasiquodo::ts::swc::ecma_ast::TsType),
            VarType::TsTypeElement => quote!(::quasiquodo::ts::swc::ecma_ast::TsTypeElement),
            VarType::Box(inner) => {
                let quoted = Self(inner);
                let tokens = quote!(#quoted);
                quote!(Box<#tokens>)
            }
            VarType::Option(inner) => {
                let quoted = Self(inner);
                let tokens = quote!(#quoted);
                quote!(Option<#tokens>)
            }
            VarType::Vec(inner) => {
                let quoted = Self(inner);
                let tokens = quote!(#quoted);
                quote!(Vec<#tokens>)
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_str;

    #[test]
    fn test_parse_simple() {
        let input: MacroInput = parse_str(r#""export type T = string;" as ModuleItem"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::ModuleItem));
        assert!(input.span.is_none());
        assert!(input.variables.is_empty());
    }

    #[test]
    fn test_parse_with_span() {
        let input: MacroInput =
            parse_str(r#"span = my_span, "name: string" as TsTypeElement"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::TsTypeElement));
        assert!(input.span.is_some());
        assert!(input.variables.is_empty());

        let input: MacroInput = parse_str(r#"span, "name: string" as TsTypeElement"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::TsTypeElement));
        let span = input.span.unwrap();
        let syn::Expr::Path(path) = &span else {
            panic!("expected `Expr::Path`; got `{span:?}`");
        };
        assert!(path.path.is_ident("span"));
    }

    #[test]
    fn test_parse_with_comments() {
        let input: MacroInput =
            parse_str(r#"comments = my_comments, "name: string" as TsTypeElement"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::TsTypeElement));
        assert!(input.comments.is_some());
        assert!(input.variables.is_empty());

        let input: MacroInput = parse_str(r#"comments, "name: string" as TsTypeElement"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::TsTypeElement));
        let comments = input.comments.unwrap();
        let syn::Expr::Path(path) = &comments else {
            panic!("expected `Expr::Path`; got `{comments:?}`");
        };
        assert!(path.path.is_ident("comments"));
    }

    #[test]
    fn test_parse_with_span_and_variables() {
        let input: MacroInput = parse_str(
            r#"span = my_span, "$name: $ty" as TsTypeElement, name: LitStr = "foo", ty: TsType = my_ty"#,
        )
        .unwrap();
        assert!(matches!(input.output_kind, OutputKind::TsTypeElement));
        assert!(input.span.is_some());
        assert_eq!(input.variables.len(), 2);
    }

    #[test]
    fn test_parse_with_variables() {
        let input: MacroInput = parse_str(
            r#""export type $Name = $T;" as ModuleItem, Name: Ident = name, T: TsType = ty"#,
        )
        .unwrap();
        assert!(input.span.is_none());
        assert_eq!(input.variables.len(), 2);
        assert_eq!(input.variables[0].name, "Name");
        assert!(matches!(input.variables[0].ty, VarType::Ident));
        assert_eq!(input.variables[1].name, "T");
        assert!(matches!(input.variables[1].ty, VarType::TsType));
    }

    #[test]
    fn test_parse_trailing_comma() {
        let input: MacroInput = parse_str(r#""export type T = string;" as ModuleItem,"#).unwrap();
        assert!(input.span.is_none());
        assert!(input.variables.is_empty());
    }

    #[test]
    fn test_parse_unknown_output_kind() {
        let result: syn::Result<MacroInput> = parse_str(r#""x" as Bogus"#);
        assert!(result.is_err());
        let msg = result.err().expect("expected error").to_string();
        assert!(msg.contains("unsupported output kind"));
    }

    #[test]
    fn test_parse_var_type_ts_type_element() {
        let vt: VarType = parse_str("TsTypeElement").unwrap();
        assert!(matches!(vt, VarType::TsTypeElement));
    }

    #[test]
    fn test_parse_var_type_vec_ts_type() {
        let vt: VarType = parse_str("Vec<TsType>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::TsType)
        ));
    }

    #[test]
    fn test_parse_var_type_vec_ts_type_element() {
        let vt: VarType = parse_str("Vec<TsTypeElement>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::TsTypeElement)
        ));
    }

    #[test]
    fn test_parse_var_type_vec_ident() {
        let vt: VarType = parse_str("Vec<Ident>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::Ident)
        ));
    }

    #[test]
    fn test_parse_var_type_class_member() {
        let vt: VarType = parse_str("ClassMember").unwrap();
        assert!(matches!(vt, VarType::ClassMember));
    }

    #[test]
    fn test_parse_var_type_param() {
        let vt: VarType = parse_str("Param").unwrap();
        assert!(matches!(vt, VarType::Param));
    }

    #[test]
    fn test_parse_var_type_stmt() {
        let vt: VarType = parse_str("Stmt").unwrap();
        assert!(matches!(vt, VarType::Stmt));
    }

    #[test]
    fn test_parse_var_type_vec_class_member() {
        let vt: VarType = parse_str("Vec<ClassMember>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::ClassMember)
        ));
    }

    #[test]
    fn test_parse_var_type_vec_param() {
        let vt: VarType = parse_str("Vec<Param>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::Param)
        ));
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
    fn test_parse_var_type_import_specifier() {
        let vt: VarType = parse_str("ImportSpecifier").unwrap();
        assert!(matches!(vt, VarType::ImportSpecifier));
    }

    #[test]
    fn test_parse_var_type_vec_import_specifier() {
        let vt: VarType = parse_str("Vec<ImportSpecifier>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::ImportSpecifier)
        ));
    }

    #[test]
    fn test_parse_var_type_export_specifier() {
        let vt: VarType = parse_str("ExportSpecifier").unwrap();
        assert!(matches!(vt, VarType::ExportSpecifier));
    }

    #[test]
    fn test_parse_var_type_vec_export_specifier() {
        let vt: VarType = syn::parse_str("Vec<ExportSpecifier>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::ExportSpecifier)
        ));
    }

    #[test]
    fn test_parse_var_type_param_or_ts_param_prop() {
        let vt: VarType = parse_str("ParamOrTsParamProp").unwrap();
        assert!(matches!(vt, VarType::ParamOrTsParamProp));
    }

    #[test]
    fn test_parse_var_type_vec_param_or_ts_param_prop() {
        let vt: VarType = parse_str("Vec<ParamOrTsParamProp>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::ParamOrTsParamProp)
        ));
    }

    #[test]
    fn test_parse_output_kind_param_or_ts_param_prop() {
        let input: MacroInput = parse_str(r#""x: string" as ParamOrTsParamProp"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::ParamOrTsParamProp));
    }

    #[test]
    fn test_parse_output_kind_import_specifier() {
        let input: MacroInput = parse_str(r#""Foo" as ImportSpecifier"#).unwrap();
        assert!(matches!(input.output_kind, OutputKind::ImportSpecifier));
    }

    #[test]
    fn test_parse_var_type_option_ts_type() {
        let vt: VarType = parse_str("Option<TsType>").unwrap();
        assert!(matches!(
            vt,
            VarType::Option(ref inner) if matches!(**inner, VarType::TsType)
        ));
    }

    #[test]
    fn test_parse_var_type_option_lit_str() {
        let vt: VarType = parse_str("Option<LitStr>").unwrap();
        assert!(matches!(
            vt,
            VarType::Option(ref inner) if matches!(**inner, VarType::LitStr)
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
    fn test_parse_var_type_option_ts_type_element() {
        let vt: VarType = parse_str("Option<TsTypeElement>").unwrap();
        assert!(matches!(
            vt,
            VarType::Option(ref inner) if matches!(**inner, VarType::TsTypeElement)
        ));
    }

    #[test]
    fn test_parse_var_type_option_class_member() {
        let vt: VarType = parse_str("Option<ClassMember>").unwrap();
        assert!(matches!(
            vt,
            VarType::Option(ref inner) if matches!(**inner, VarType::ClassMember)
        ));
    }

    #[test]
    fn test_parse_var_type_decl() {
        let vt: VarType = parse_str("Decl").unwrap();
        assert!(matches!(vt, VarType::Decl));
    }

    #[test]
    fn test_parse_var_type_vec_decl() {
        let vt: VarType = parse_str("Vec<Decl>").unwrap();
        assert!(matches!(
            vt,
            VarType::Vec(ref inner) if matches!(**inner, VarType::Decl)
        ));
    }

    #[test]
    fn test_parse_var_type_option_decl() {
        let vt: VarType = parse_str("Option<Decl>").unwrap();
        assert!(matches!(
            vt,
            VarType::Option(ref inner) if matches!(**inner, VarType::Decl)
        ));
    }

    #[test]
    fn test_parse_var_type_jsdoc() {
        let vt: VarType = parse_str("JsDoc").unwrap();
        assert!(matches!(vt, VarType::JsDoc));
    }

    #[test]
    fn test_parse_var_type_option_jsdoc() {
        let vt: VarType = parse_str("Option<JsDoc>").unwrap();
        assert!(matches!(
            vt,
            VarType::Option(ref inner) if matches!(**inner, VarType::JsDoc)
        ));
    }
}
