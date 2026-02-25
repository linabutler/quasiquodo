use proc_macro2::Span;
use swc_ecma_ast::*;
use syn::parse_quote;

use crate::{context::Context, input::VarType, lift::unsplice};

use super::{
    CodeFragment, Lift, SpliceIdent, impl_lift_for_newtype_enum, impl_lift_for_struct,
    impl_lift_for_unit_enum, lift_variants, splice_idents,
};

impl_lift_for_newtype_enum!(ModuleItem, [ModuleDecl, Stmt]);

impl_lift_for_newtype_enum!(
    ModuleDecl,
    [
        Import,
        ExportDecl,
        ExportNamed,
        ExportDefaultDecl,
        ExportDefaultExpr,
        ExportAll,
        TsImportEquals,
        TsExportAssignment,
        TsNamespaceExport
    ]
);

impl_lift_for_struct!(ExportDecl, [span, decl]);

impl SpliceIdent for ImportSpecifier {
    fn splice_ident(&self, context: &Context) -> Option<syn::Expr> {
        let ImportSpecifier::Named(named) = self else {
            return None;
        };
        if named.imported.is_some() || named.is_type_only {
            return None;
        }
        let var = context.stand_in(&named.local.sym)?;
        if let VarType::Vec(v) | VarType::Option(v) = &var.ty
            && matches!(**v, VarType::Ident)
        {
            let var_ident = var.to_tokens();
            let sp = context.span();
            Some(parse_quote!(#var_ident.into_iter().map(|__id| {
                ::quasiquodo::ts::swc::ecma_ast::ImportSpecifier::Named(
                    ::quasiquodo::ts::swc::ecma_ast::ImportNamedSpecifier {
                        span: #sp,
                        local: __id,
                        imported: None,
                        is_type_only: false,
                    },
                )
            })))
        } else {
            None
        }
    }
}

impl SpliceIdent for ExportSpecifier {
    fn splice_ident(&self, context: &Context) -> Option<syn::Expr> {
        let ExportSpecifier::Named(named) = self else {
            return None;
        };
        if named.exported.is_some() || named.is_type_only {
            return None;
        }
        let ModuleExportName::Ident(ident) = &named.orig else {
            return None;
        };
        let var = context.stand_in(&ident.sym)?;
        if let VarType::Vec(v) | VarType::Option(v) = &var.ty
            && matches!(**v, VarType::Ident)
        {
            let var_ident = var.to_tokens();
            let sp = context.span();
            Some(parse_quote!(#var_ident.into_iter().map(|__id| {
                ::quasiquodo::ts::swc::ecma_ast::ExportSpecifier::Named(
                    ::quasiquodo::ts::swc::ecma_ast::ExportNamedSpecifier {
                        span: #sp,
                        orig: ::quasiquodo::ts::swc::ecma_ast::ModuleExportName::Ident(__id),
                        exported: None,
                        is_type_only: false,
                    },
                )
            })))
        } else {
            None
        }
    }
}

/// Custom implementation to support splicing `Vec<Ident>`s into
/// the imported module specifier.
impl Lift for ImportDecl {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let Self {
            span,
            specifiers,
            src,
            type_only,
            with,
            phase,
        } = self;

        let span = unsplice!(span.lift(context)?);
        let src = unsplice!(src.lift(context)?);
        let type_only = unsplice!(type_only.lift(context)?);
        let with = unsplice!(with.lift(context)?);
        let phase = unsplice!(phase.lift(context)?);

        let specifiers_expr = splice_idents(specifiers, context)?;

        Ok(CodeFragment::Single(
            parse_quote!(::quasiquodo::ts::swc::ecma_ast::ImportDecl {
                span: #span,
                specifiers: #specifiers_expr,
                src: #src,
                type_only: #type_only,
                with: #with,
                phase: #phase,
            }),
        ))
    }
}

/// Custom implementation to support splicing `Vec<Ident>`s into
/// the exported module specifier.
impl Lift for NamedExport {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let Self {
            span,
            specifiers,
            src,
            type_only,
            with,
        } = self;

        let span = unsplice!(span.lift(context)?);
        let src = unsplice!(src.lift(context)?);
        let type_only = unsplice!(type_only.lift(context)?);
        let with = unsplice!(with.lift(context)?);

        let specifiers_expr = splice_idents(specifiers, context)?;

        Ok(CodeFragment::Single(
            parse_quote!(::quasiquodo::ts::swc::ecma_ast::NamedExport {
                span: #span,
                specifiers: #specifiers_expr,
                src: #src,
                type_only: #type_only,
                with: #with,
            }),
        ))
    }
}

impl_lift_for_struct!(ExportDefaultDecl, [span, decl]);

impl_lift_for_struct!(ExportDefaultExpr, [span, expr]);

impl_lift_for_struct!(ExportAll, [span, src, type_only, with]);

impl_lift_for_struct!(
    TsImportEqualsDecl,
    [span, is_export, is_type_only, id, module_ref]
);

impl_lift_for_struct!(TsExportAssignment, [span, expr]);

impl_lift_for_struct!(TsNamespaceExportDecl, [span, id]);

impl_lift_for_newtype_enum!(DefaultDecl, [Class, Fn, TsInterfaceDecl]);

impl_lift_for_newtype_enum!(TsModuleRef, [TsEntityName, TsExternalModuleRef]);

impl_lift_for_struct!(TsExternalModuleRef, [span, expr]);

impl_lift_for_unit_enum!(ImportPhase, [Evaluation, Source, Defer]);

impl_lift_for_newtype_enum!(ImportSpecifier, [Named, Default, Namespace]);

impl_lift_for_struct!(ImportNamedSpecifier, [span, local, imported, is_type_only]);

impl_lift_for_struct!(ImportDefaultSpecifier, [span, local]);

impl_lift_for_struct!(ImportStarAsSpecifier, [span, local]);

impl_lift_for_newtype_enum!(ExportSpecifier, [Namespace, Default, Named]);

impl_lift_for_struct!(ExportNamedSpecifier, [span, orig, exported, is_type_only]);

impl_lift_for_struct!(ExportNamespaceSpecifier, [span, name]);

impl_lift_for_struct!(ExportDefaultSpecifier, [exported]);

impl_lift_for_newtype_enum!(ModuleExportName, [Ident, Str]);

impl_lift_for_struct!(ObjectLit, [span, props]);

impl_lift_for_newtype_enum!(PropOrSpread, [Spread, Prop]);

impl_lift_for_struct!(SpreadElement, [dot3_token, expr]);

impl_lift_for_newtype_enum!(Prop, [Shorthand, KeyValue, Assign, Getter, Setter, Method]);

impl_lift_for_struct!(KeyValueProp, [key, value]);

impl_lift_for_struct!(AssignProp, [span, key, value]);

impl_lift_for_struct!(GetterProp, [span, key, type_ann, body]);

impl_lift_for_struct!(SetterProp, [span, key, this_param, param, body]);

impl_lift_for_struct!(MethodProp, [key, function]);

/// Custom implementation to rewrite computed properties with string variables
/// as bare identifier properties where valid.
impl Lift for PropName {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let PropName::Computed(computed) = self else {
            return lift_variants!(self, context, PropName, [Ident, Str, Num, BigInt, Computed]);
        };
        // Check if this is a stand-in for a string variable
        // inserted during preprocessing.
        let var = if let Expr::Lit(Lit::Str(s)) = &*computed.expr
            && let Some(value) = s.value.as_str()
            && let Some(var) = context.stand_in(value)
            && var.ty.is_str()
        {
            Some(var)
        } else if let Expr::Ident(ident) = &*computed.expr
            && let Some(var) = context.stand_in(&ident.sym)
            && var.ty.is_str()
        {
            Some(var)
        } else {
            None
        };
        match var {
            // Generate conditional code that chooses between `Ident` and `Str`
            // (not `Computed`, since we have the static value), depending on
            // whether the string variable's value would be a valid identifier.
            Some(var) => {
                let var_ident = var.to_tokens();
                let span_expr = context.span();
                Ok(CodeFragment::Single(parse_quote!({
                    let name = #var_ident;
                    if ::quasiquodo::ts::swc::ecma_utils::is_valid_prop_ident(&name) {
                        ::quasiquodo::ts::swc::ecma_ast::PropName::Ident(::quasiquodo::ts::swc::ecma_ast::IdentName {
                            span: #span_expr,
                            sym: name.into(),
                        })
                    } else {
                        ::quasiquodo::ts::swc::ecma_ast::PropName::Str(::quasiquodo::ts::swc::ecma_ast::Str {
                            span: #span_expr,
                            value: name.into(),
                            raw: None,
                        })
                    }
                })))
            }
            // In all other cases, emit the computed property verbatim.
            None => {
                let val = unsplice!(Lift::lift(computed, context)?);
                Ok(CodeFragment::Single(
                    parse_quote!(::quasiquodo::ts::swc::ecma_ast::PropName::Computed(#val)),
                ))
            }
        }
    }
}

impl_lift_for_struct!(ComputedPropName, [span, expr]);
