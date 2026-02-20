use proc_macro2::Span;
use swc_ecma_ast::*;
use syn::parse_quote;

use crate::{context::Context, input::VarType};

use super::{
    CodeFragment, Lift, impl_lift_for_newtype_enum, impl_lift_for_struct, impl_lift_for_unit_enum,
    lift_variants, unsplice,
};

impl Lift for TsType {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        // Check for placeholder type references that stand in
        // for `TsType` variables.
        if let TsType::TsTypeRef(TsTypeRef {
            type_name: TsEntityName::Ident(ident),
            type_params: None,
            ..
        }) = self
            && let Some(var) = context.placeholder(&ident.sym)
        {
            let var_ident = var.to_tokens();
            let span_expr = context.span();
            match &var.ty {
                VarType::Box(inner) if **inner == VarType::TsType => {
                    return Ok(CodeFragment::Single(parse_quote!(*#var_ident)));
                }
                VarType::TsType => {
                    return Ok(CodeFragment::Single(parse_quote!(#var_ident)));
                }
                VarType::LitStr => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(::quasiquodo::ts::swc::ecma_ast::TsType::TsLitType(
                            ::quasiquodo::ts::swc::ecma_ast::TsLitType {
                                span: #span_expr,
                                lit: ::quasiquodo::ts::swc::ecma_ast::TsLit::Str(::quasiquodo::ts::swc::ecma_ast::Str {
                                    span: #span_expr,
                                    value: (#var_ident).into(),
                                    raw: None,
                                }),
                            }
                        )),
                    ));
                }
                VarType::LitNum => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(::quasiquodo::ts::swc::ecma_ast::TsType::TsLitType(
                            ::quasiquodo::ts::swc::ecma_ast::TsLitType {
                                span: #span_expr,
                                lit: ::quasiquodo::ts::swc::ecma_ast::TsLit::Number(::quasiquodo::ts::swc::ecma_ast::Number {
                                    span: #span_expr,
                                    value: #var_ident,
                                    raw: None,
                                }),
                            }
                        )),
                    ));
                }
                VarType::LitBool => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(::quasiquodo::ts::swc::ecma_ast::TsType::TsLitType(
                            ::quasiquodo::ts::swc::ecma_ast::TsLitType {
                                span: #span_expr,
                                lit: ::quasiquodo::ts::swc::ecma_ast::TsLit::Bool(::quasiquodo::ts::swc::ecma_ast::Bool {
                                    span: #span_expr,
                                    value: #var_ident,
                                }),
                            }
                        )),
                    ));
                }
                VarType::Vec(_) | VarType::Option(_) => {
                    return Ok(CodeFragment::Splice(parse_quote!(#var_ident.into_iter())));
                }
                _ => (),
            }
        }

        // Check for placeholder string literals that stand in for
        // `Vec<LitStr>` variables. Preprocessing wraps `LitStr`
        // placeholders in quotes, so SWC parses them as
        // `TsLitType { lit: Str(s) }`.
        if let TsType::TsLitType(TsLitType {
            lit: TsLit::Str(s), ..
        }) = self
            && let Some(value) = s.value.as_str()
            && let Some(var) = context.placeholder(value)
            && let VarType::Vec(inner) | VarType::Option(inner) = &var.ty
            && matches!(**inner, VarType::LitStr)
        {
            let var_ident = var.to_tokens();
            let span_expr = context.span();
            return Ok(CodeFragment::Splice(parse_quote!(
                #var_ident.into_iter().map(|__s| {
                    Box::new(::quasiquodo::ts::swc::ecma_ast::TsType::TsLitType(
                        ::quasiquodo::ts::swc::ecma_ast::TsLitType {
                            span: #span_expr,
                            lit: ::quasiquodo::ts::swc::ecma_ast::TsLit::Str(::quasiquodo::ts::swc::ecma_ast::Str {
                                span: #span_expr,
                                value: __s.into(),
                                raw: None,
                            }),
                        }
                    ))
                })
            )));
        }

        lift_variants!(
            self,
            context,
            TsType,
            [
                TsKeywordType,
                TsThisType,
                TsFnOrConstructorType,
                TsTypeRef,
                TsTypeQuery,
                TsTypeLit,
                TsArrayType,
                TsTupleType,
                TsOptionalType,
                TsRestType,
                TsUnionOrIntersectionType,
                TsConditionalType,
                TsInferType,
                TsParenthesizedType,
                TsTypeOperator,
                TsIndexedAccessType,
                TsMappedType,
                TsLitType,
                TsTypePredicate,
                TsImportType
            ]
        )
    }
}

impl_lift_for_struct!(TsKeywordType, [span, kind]);

impl_lift_for_struct!(TsTypeRef, [span, type_name, type_params]);

impl_lift_for_struct!(TsArrayType, [span, elem_type]);

impl_lift_for_struct!(TsTypeLit, [span, members]);

impl_lift_for_struct!(TsLitType, [span, lit]);

impl_lift_for_struct!(TsParenthesizedType, [span, type_ann]);

impl_lift_for_struct!(TsThisType, [span]);

impl_lift_for_struct!(TsOptionalType, [span, type_ann]);

impl_lift_for_struct!(TsRestType, [span, type_ann]);

impl_lift_for_struct!(TsTupleType, [span, elem_types]);

impl_lift_for_struct!(TsTupleElement, [span, label, ty]);

impl_lift_for_struct!(TsUnionType, [span, types]);

impl_lift_for_struct!(TsIntersectionType, [span, types]);

impl_lift_for_newtype_enum!(TsFnOrConstructorType, [TsFnType, TsConstructorType]);

impl_lift_for_struct!(TsFnType, [span, params, type_params, type_ann]);

impl_lift_for_struct!(
    TsConstructorType,
    [span, params, type_params, type_ann, is_abstract]
);

impl_lift_for_struct!(TsTypeQuery, [span, expr_name, type_args]);

impl_lift_for_newtype_enum!(TsTypeQueryExpr, [TsEntityName, Import]);

impl_lift_for_struct!(
    TsConditionalType,
    [span, check_type, extends_type, true_type, false_type]
);

impl_lift_for_struct!(TsInferType, [span, type_param]);

impl_lift_for_struct!(TsTypeOperator, [span, op, type_ann]);

impl_lift_for_unit_enum!(TsTypeOperatorOp, [KeyOf, Unique, ReadOnly]);

impl_lift_for_struct!(TsIndexedAccessType, [span, readonly, obj_type, index_type]);

impl_lift_for_struct!(
    TsMappedType,
    [span, readonly, type_param, name_type, optional, type_ann]
);

impl_lift_for_unit_enum!(TruePlusMinus, [True, Plus, Minus]);

impl_lift_for_struct!(TsTypePredicate, [span, asserts, param_name, type_ann]);

impl_lift_for_newtype_enum!(TsThisTypeOrIdent, [TsThisType, Ident]);

impl_lift_for_struct!(TsImportType, [span, arg, qualifier, type_args, attributes]);

impl_lift_for_struct!(TsImportCallOptions, [span, with]);

impl_lift_for_newtype_enum!(TsUnionOrIntersectionType, [TsUnionType, TsIntersectionType]);

impl_lift_for_newtype_enum!(TsEntityName, [Ident, TsQualifiedName]);

impl_lift_for_struct!(TsQualifiedName, [span, left, right]);

impl_lift_for_struct!(TsTypeParamInstantiation, [span, params]);

impl_lift_for_newtype_enum!(TsLit, [Str, Number, Bool, BigInt, Tpl]);

impl_lift_for_struct!(TsTplLitType, [span, types, quasis]);

impl_lift_for_struct!(TplElement, [span, tail, cooked, raw]);

impl_lift_for_unit_enum!(
    TsKeywordTypeKind,
    [
        TsAnyKeyword,
        TsUnknownKeyword,
        TsNumberKeyword,
        TsObjectKeyword,
        TsBooleanKeyword,
        TsBigIntKeyword,
        TsStringKeyword,
        TsSymbolKeyword,
        TsVoidKeyword,
        TsUndefinedKeyword,
        TsNullKeyword,
        TsNeverKeyword,
        TsIntrinsicKeyword
    ]
);

impl_lift_for_newtype_enum!(
    TsTypeElement,
    [
        TsCallSignatureDecl,
        TsConstructSignatureDecl,
        TsPropertySignature,
        TsGetterSignature,
        TsSetterSignature,
        TsMethodSignature,
        TsIndexSignature
    ]
);

impl Lift for TsPropertySignature {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let Self {
            span,
            readonly,
            key,
            computed,
            optional,
            type_ann,
        } = self;

        let span = unsplice!(Lift::lift(span, context)?);
        let readonly = unsplice!(Lift::lift(readonly, context)?);

        // Simplify keys that are valid property identifiers:
        // when a `LitStr` placeholder is in the key position, emit
        // conditional code that uses `Expr::Ident` for valid
        // identifiers (e.g., `bare_name`), and `Expr::Lit` for
        // non-identifiers (e.g., `"kebab-name"`).
        let var = if let Expr::Lit(swc_ecma_ast::Lit::Str(s)) = key.as_ref()
            && let Some(value) = s.value.as_str()
            && let Some(var) = context.placeholder(value)
            && matches!(var.ty, VarType::LitStr)
        {
            Some(var)
        } else if let Expr::Ident(ident) = key.as_ref()
            && let Some(var) = context.placeholder(&ident.sym)
            && matches!(var.ty, VarType::LitStr)
        {
            Some(var)
        } else {
            None
        };
        let key = match var {
            // Generate conditional code that chooses between `Ident`
            // and `Lit`, depending on whether the string variable's
            // value would be a valid identifier.
            Some(var) => {
                let var_ident = var.to_tokens();
                let span_expr = context.span();
                parse_quote!(Box::new({
                    let name = #var_ident;
                    if ::quasiquodo::ts::swc::ecma_utils::is_valid_prop_ident(name) {
                        ::quasiquodo::ts::swc::ecma_ast::Expr::Ident(
                            ::quasiquodo::ts::swc::ecma_ast::Ident::new_no_ctxt(
                                name.into(),
                                #span_expr,
                            )
                        )
                    } else {
                        ::quasiquodo::ts::swc::ecma_ast::Expr::Lit(::quasiquodo::ts::swc::ecma_ast::Lit::Str(
                            ::quasiquodo::ts::swc::ecma_ast::Str {
                                span: #span_expr,
                                value: name.into(),
                                raw: None,
                            }
                        ))
                    }
                }))
            }
            // In all other cases, emit the key expression verbatim.
            None => unsplice!(Lift::lift(key, context)?),
        };

        let computed = unsplice!(Lift::lift(computed, context)?);
        let optional = unsplice!(Lift::lift(optional, context)?);
        let type_ann = unsplice!(Lift::lift(type_ann, context)?);

        Ok(CodeFragment::Single(
            parse_quote!(::quasiquodo::ts::swc::ecma_ast::TsPropertySignature {
                span: #span,
                readonly: #readonly,
                key: #key,
                computed: #computed,
                optional: #optional,
                type_ann: #type_ann,
            }),
        ))
    }
}

impl_lift_for_struct!(
    TsMethodSignature,
    [span, key, computed, optional, params, type_ann, type_params]
);

impl_lift_for_struct!(
    TsIndexSignature,
    [params, type_ann, readonly, is_static, span]
);

impl_lift_for_struct!(TsCallSignatureDecl, [span, params, type_ann, type_params]);

impl_lift_for_struct!(
    TsConstructSignatureDecl,
    [span, params, type_ann, type_params]
);

impl_lift_for_struct!(TsGetterSignature, [span, key, computed, type_ann]);

impl_lift_for_struct!(TsSetterSignature, [span, key, computed, param]);

impl_lift_for_newtype_enum!(TsFnParam, [Ident, Array, Rest, Object]);

impl_lift_for_struct!(BindingIdent, [id, type_ann]);

impl_lift_for_struct!(ArrayPat, [span, elems, optional, type_ann]);

impl_lift_for_struct!(ObjectPat, [span, props, optional, type_ann]);

impl_lift_for_struct!(RestPat, [span, dot3_token, arg, type_ann]);

impl_lift_for_struct!(AssignPat, [span, left, right]);

impl_lift_for_struct!(Invalid, [span]);

impl_lift_for_newtype_enum!(ObjectPatProp, [KeyValue, Assign, Rest]);

impl_lift_for_struct!(KeyValuePatProp, [key, value]);

impl_lift_for_struct!(AssignPatProp, [span, key, value]);

impl_lift_for_newtype_enum!(Pat, [Ident, Array, Rest, Object, Assign, Invalid, Expr]);
