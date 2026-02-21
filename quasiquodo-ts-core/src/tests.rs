use quote::quote;
use syn::parse_quote;

use crate::expand;

/// Expands a `ts_quote!` invocation, then parses the resulting
/// token stream as a [`syn::Expr`].
fn expand_expr(input: proc_macro2::TokenStream) -> syn::Expr {
    let tokens = expand(input);
    parse_quote!(#tokens)
}

// MARK: `span` arguments

#[test]
fn test_expand_with_span_parameter() {
    let actual = expand_expr(quote!(span = my_span, "name: string" as TsTypeElement));
    let expected: syn::Expr = parse_quote! {{
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: my_span,
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(::quasiquodo::ts::swc::atoms::atom!("name"), my_span,)
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: my_span,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: my_span,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

#[test]
fn test_expand_with_span_and_variables() {
    let actual = expand_expr(quote!(
        span = my_span, "@{name}: @{ty}" as TsTypeElement, name: LitStr = "foo", ty: TsType = my_ty
    ));
    let expected: syn::Expr = parse_quote! {{
        let quote_var_name: &str = "foo";
        let quote_var_ty: ::quasiquodo::ts::swc::ecma_ast::TsType = my_ty;
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: my_span,
            readonly: false,
            key: Box::new({
                let name = quote_var_name.clone();
                if ::quasiquodo::ts::swc::ecma_utils::is_valid_prop_ident(name) {
                    ::quasiquodo::ts::swc::ecma_ast::Expr::Ident(::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                        name.into(),
                        my_span,
                    ))
                } else {
                    ::quasiquodo::ts::swc::ecma_ast::Expr::Lit(::quasiquodo::ts::swc::ecma_ast::Lit::Str(
                        ::quasiquodo::ts::swc::ecma_ast::Str {
                            span: my_span,
                            value: name.into(),
                            raw: None,
                        }
                    ))
                }
            }),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: my_span,
                type_ann: Box::new(quote_var_ty.clone()),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

#[test]
fn test_expand_without_span_uses_dummy_span() {
    let actual = expand_expr(quote!("name: string" as TsTypeElement));
    let expected: syn::Expr = parse_quote! {{
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: ::quasiquodo::ts::swc::common::DUMMY_SP,
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                    ::quasiquodo::ts::swc::atoms::atom!("name"),
                    ::quasiquodo::ts::swc::common::DUMMY_SP,
                )
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

// MARK: `comments` arguments

#[test]
fn test_expand_static_doc_comment() {
    let actual = expand_expr(quote!(
        comments = my_comments,
        "/** Fixed. */ name: string" as TsTypeElement
    ));
    let expected: syn::Expr = parse_quote! {{
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: ::quasiquodo::ts::Comments::span_with_comment(&my_comments, "* Fixed. ",),
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                    ::quasiquodo::ts::swc::atoms::atom!("name"),
                    ::quasiquodo::ts::swc::common::DUMMY_SP,
                )
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

#[test]
fn test_expand_dynamic_doc_comment() {
    let actual = expand_expr(quote!(
        comments = my_comments,
        "/** @{desc} */ name: string" as TsTypeElement,
        desc: LitStr = "hello"
    ));
    let expected: syn::Expr = parse_quote! {{
        let quote_var_desc: &str = "hello";
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: ::quasiquodo::ts::Comments::span_with_comment(&my_comments, format!("* {} ", quote_var_desc),),
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                    ::quasiquodo::ts::swc::atoms::atom!("name"),
                    ::quasiquodo::ts::swc::common::DUMMY_SP,
                )
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

// MARK: `ImportSpecifier` output kind
//
// `ImportSpecifier` doesn't implement [`swc_ecma_codegen::Node`],
// so it can't be tested in the codegen integration tests.

#[test]
fn test_expand_import_specifier_named() {
    let actual = expand_expr(quote!("Foo" as ImportSpecifier));
    let expected: syn::Expr = parse_quote! {{
        ::quasiquodo::ts::swc::ecma_ast::ImportSpecifier::Named(::quasiquodo::ts::swc::ecma_ast::ImportNamedSpecifier {
            span: ::quasiquodo::ts::swc::common::DUMMY_SP,
            local: ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                ::quasiquodo::ts::swc::atoms::atom!("Foo"),
                ::quasiquodo::ts::swc::common::DUMMY_SP,
            ),
            imported: None,
            is_type_only: false,
        })
    }};
    assert_eq!(actual, expected);
}

// MARK: `JsDoc` variable

#[test]
fn test_expand_jsdoc_variable() {
    let actual = expand_expr(quote!(
        comments = my_comments,
        "@{doc} name: string" as TsTypeElement,
        doc: JsDoc = my_doc
    ));
    let expected: syn::Expr = parse_quote! {{
        let quote_var_doc: ::quasiquodo::ts::JsDoc = my_doc;
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: ::quasiquodo::ts::Comments::span_with_comment(&my_comments, format!("* {} ", quote_var_doc.raw_text()),),
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                    ::quasiquodo::ts::swc::atoms::atom!("name"),
                    ::quasiquodo::ts::swc::common::DUMMY_SP,
                )
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

#[test]
fn test_expand_option_jsdoc_variable() {
    let actual = expand_expr(quote!(
        comments = my_comments,
        "@{doc} name: string" as TsTypeElement,
        doc: Option<JsDoc> = my_doc
    ));
    let expected: syn::Expr = parse_quote! {{
        let quote_var_doc: Option<::quasiquodo::ts::JsDoc> = my_doc;
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: match quote_var_doc {
                Some(ref doc) => ::quasiquodo::ts::Comments::span_with_comment(&my_comments, format!("* {} ", doc.raw_text()),),
                None => ::quasiquodo::ts::swc::common::DUMMY_SP,
            },
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                    ::quasiquodo::ts::swc::atoms::atom!("name"),
                    ::quasiquodo::ts::swc::common::DUMMY_SP,
                )
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

#[test]
fn test_expand_jsdoc_variable_without_comments() {
    // Without a `comments` argument to collect them, `JsDoc` variables
    // become dummy spans, effectively dropping them.
    let actual = expand_expr(quote!(
        "@{doc} name: string" as TsTypeElement,
        doc: JsDoc = my_doc
    ));
    let expected: syn::Expr = parse_quote! {{
        let quote_var_doc: ::quasiquodo::ts::JsDoc = my_doc;
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: ::quasiquodo::ts::swc::common::DUMMY_SP,
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                    ::quasiquodo::ts::swc::atoms::atom!("name"),
                    ::quasiquodo::ts::swc::common::DUMMY_SP,
                )
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

// MARK: `JsDoc` embedded in comment

#[test]
fn test_expand_jsdoc_embedded_in_comment() {
    let actual = expand_expr(quote!(
        comments = my_comments,
        "/** See @{doc}. */ name: string" as TsTypeElement,
        doc: JsDoc = my_doc
    ));
    let expected: syn::Expr = parse_quote! {{
        let quote_var_doc: ::quasiquodo::ts::JsDoc = my_doc;
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: ::quasiquodo::ts::Comments::span_with_comment(&my_comments, format!("* See {}. ", quote_var_doc.raw_text()),),
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                    ::quasiquodo::ts::swc::atoms::atom!("name"),
                    ::quasiquodo::ts::swc::common::DUMMY_SP,
                )
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

#[test]
fn test_expand_option_jsdoc_embedded_in_comment() {
    let actual = expand_expr(quote!(
        comments = my_comments,
        "/** See @{doc}. */ name: string" as TsTypeElement,
        doc: Option<JsDoc> = my_doc
    ));
    let expected: syn::Expr = parse_quote! {{
        let quote_var_doc: Option<::quasiquodo::ts::JsDoc> = my_doc;
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: ::quasiquodo::ts::Comments::span_with_comment(&my_comments, format!("* See {}. ", quote_var_doc.as_ref().map(|d| d.raw_text()).unwrap_or_default()),),
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                    ::quasiquodo::ts::swc::atoms::atom!("name"),
                    ::quasiquodo::ts::swc::common::DUMMY_SP,
                )
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

// MARK: `Option<LitStr>` sole placeholder

#[test]
fn test_expand_option_litstr_sole_placeholder() {
    let actual = expand_expr(quote!(
        comments = my_comments,
        "/** @{desc} */ name: string" as TsTypeElement,
        desc: Option<LitStr> = my_desc
    ));
    let expected: syn::Expr = parse_quote! {{
        let quote_var_desc: Option<&str> = my_desc;
        ::quasiquodo::ts::swc::ecma_ast::TsTypeElement::TsPropertySignature(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
            span: match quote_var_desc {
                Some(ref doc) => ::quasiquodo::ts::Comments::span_with_comment(&my_comments, format!("* {} ", doc),),
                None => ::quasiquodo::ts::swc::common::DUMMY_SP,
            },
            readonly: false,
            key: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                    ::quasiquodo::ts::swc::atoms::atom!("name"),
                    ::quasiquodo::ts::swc::common::DUMMY_SP,
                )
            )),
            computed: false,
            optional: false,
            type_ann: Some(Box::new(::quasiquodo::ts::swc::ecma_ast::TsTypeAnn {
                span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                type_ann: Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsKeywordType(
                    ::quasiquodo::ts::swc::ecma_ast::TsKeywordType {
                        span: ::quasiquodo::ts::swc::common::DUMMY_SP,
                        kind: ::quasiquodo::ts::swc::ecma_ast::TsKeywordTypeKind::TsStringKeyword,
                    }
                )),
            })),
        })
    }};
    assert_eq!(actual, expected);
}

#[test]
fn test_expand_import_specifier_with_ident_variable() {
    let actual = expand_expr(quote!(
        "@{local}" as ImportSpecifier, local: Ident = my_ident
    ));
    let expected: syn::Expr = parse_quote! {{
        let quote_var_local: ::quasiquodo::ts::swc::ecma_ast::Ident = my_ident;
        ::quasiquodo::ts::swc::ecma_ast::ImportSpecifier::Named(::quasiquodo::ts::swc::ecma_ast::ImportNamedSpecifier {
            span: ::quasiquodo::ts::swc::common::DUMMY_SP,
            local: quote_var_local.clone(),
            imported: None,
            is_type_only: false,
        })
    }};
    assert_eq!(actual, expected);
}
