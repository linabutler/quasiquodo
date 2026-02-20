use proc_macro2::Span;
use swc_ecma_ast::*;
use syn::parse_quote;

use crate::{context::Context, input::VarType, lift::unsplice};

use super::{
    CodeFragment, Lift, SpliceIdent, impl_lift_for_newtype_enum, impl_lift_for_struct,
    impl_lift_for_unit_enum, lift_variants, splice_idents,
};

impl_lift_for_newtype_enum!(
    Decl,
    [
        Class,
        Fn,
        Var,
        Using,
        TsInterface,
        TsTypeAlias,
        TsEnum,
        TsModule
    ]
);

impl_lift_for_struct!(ClassDecl, [ident, declare, class]);

impl_lift_for_struct!(
    Class,
    [
        span,
        ctxt,
        decorators,
        body,
        super_class,
        is_abstract,
        type_params,
        super_type_params,
        implements
    ]
);

impl_lift_for_newtype_enum!(
    ClassMember,
    [
        Constructor,
        Method,
        PrivateMethod,
        ClassProp,
        PrivateProp,
        TsIndexSignature,
        Empty,
        StaticBlock,
        AutoAccessor
    ]
);

impl_lift_for_struct!(
    Constructor,
    [span, ctxt, key, params, body, accessibility, is_optional]
);

impl_lift_for_newtype_enum!(ParamOrTsParamProp, [TsParamProp, Param]);

impl_lift_for_struct!(
    TsParamProp,
    [
        span,
        decorators,
        accessibility,
        is_override,
        readonly,
        param
    ]
);

impl_lift_for_newtype_enum!(TsParamPropParam, [Ident, Assign]);

impl_lift_for_struct!(
    ClassMethod,
    [
        span,
        key,
        function,
        kind,
        is_static,
        accessibility,
        is_abstract,
        is_optional,
        is_override
    ]
);

impl_lift_for_struct!(
    PrivateMethod,
    [
        span,
        key,
        function,
        kind,
        is_static,
        accessibility,
        is_abstract,
        is_optional,
        is_override
    ]
);

impl_lift_for_struct!(PrivateName, [span, name]);

impl_lift_for_unit_enum!(MethodKind, [Method, Getter, Setter]);

impl_lift_for_unit_enum!(Accessibility, [Public, Protected, Private]);

impl_lift_for_struct!(
    ClassProp,
    [
        span,
        key,
        value,
        type_ann,
        is_static,
        decorators,
        accessibility,
        is_abstract,
        is_optional,
        is_override,
        readonly,
        declare,
        definite
    ]
);

impl_lift_for_struct!(
    PrivateProp,
    [
        span,
        ctxt,
        key,
        value,
        type_ann,
        is_static,
        decorators,
        accessibility,
        is_optional,
        is_override,
        readonly,
        definite
    ]
);

impl_lift_for_struct!(StaticBlock, [span, body]);

impl_lift_for_struct!(
    AutoAccessor,
    [
        span,
        key,
        value,
        type_ann,
        is_static,
        decorators,
        accessibility,
        is_abstract,
        is_override,
        definite
    ]
);

impl_lift_for_newtype_enum!(Key, [Private, Public]);

impl_lift_for_struct!(EmptyStmt, [span]);

impl_lift_for_struct!(Decorator, [span, expr]);

impl_lift_for_struct!(FnDecl, [ident, declare, function]);

impl_lift_for_struct!(
    Function,
    [
        params,
        decorators,
        span,
        ctxt,
        body,
        is_generator,
        is_async,
        type_params,
        return_type
    ]
);

impl_lift_for_struct!(Param, [span, decorators, pat]);

impl_lift_for_struct!(VarDecl, [span, ctxt, kind, declare, decls]);

impl_lift_for_struct!(VarDeclarator, [span, name, init, definite]);

impl_lift_for_unit_enum!(VarDeclKind, [Var, Let, Const]);

impl_lift_for_struct!(TsEnumDecl, [span, declare, is_const, id, members]);

impl_lift_for_struct!(TsEnumMember, [span, id, init]);

impl_lift_for_newtype_enum!(TsEnumMemberId, [Ident, Str]);

impl_lift_for_struct!(UsingDecl, [span, is_await, decls]);

impl_lift_for_struct!(TsModuleDecl, [span, declare, global, namespace, id, body]);

impl_lift_for_struct!(TsModuleBlock, [span, body]);

impl_lift_for_struct!(TsNamespaceDecl, [span, declare, global, id, body]);

impl_lift_for_newtype_enum!(TsModuleName, [Ident, Str]);

impl_lift_for_newtype_enum!(TsNamespaceBody, [TsModuleBlock, TsNamespaceDecl]);

impl_lift_for_struct!(TsTypeAliasDecl, [span, declare, id, type_params, type_ann]);

impl SpliceIdent for TsExprWithTypeArgs {
    fn splice_ident(&self, context: &Context) -> Option<syn::Expr> {
        let Expr::Ident(ident) = self.expr.as_ref() else {
            return None;
        };
        if self.type_args.is_some() {
            return None;
        }
        let var = context.placeholder(&ident.sym)?;
        if let VarType::Vec(v) | VarType::Option(v) = &var.ty
            && matches!(**v, VarType::Ident)
        {
            let var_ident = var.to_tokens();
            let span_expr = context.span();
            Some(parse_quote!(#var_ident.into_iter().map(|id| {
                ::quasiquodo::ts::swc::ecma_ast::TsExprWithTypeArgs {
                    span: #span_expr,
                    expr: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Ident(id)),
                    type_args: None,
                }
            })))
        } else {
            None
        }
    }
}

/// Custom implementation to support splicing `Vec<Ident>`s into
/// the `extends` clause.
impl Lift for TsInterfaceDecl {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let Self {
            span,
            id,
            declare,
            type_params,
            extends,
            body,
        } = self;

        let span = unsplice!(span.lift(context)?);
        let id = unsplice!(id.lift(context)?);
        let declare = unsplice!(declare.lift(context)?);
        let type_params = unsplice!(type_params.lift(context)?);
        let body = unsplice!(body.lift(context)?);

        let extends_expr = splice_idents(extends, context)?;

        Ok(CodeFragment::Single(
            parse_quote!(::quasiquodo::ts::swc::ecma_ast::TsInterfaceDecl {
                span: #span,
                id: #id,
                declare: #declare,
                type_params: #type_params,
                extends: #extends_expr,
                body: #body,
            }),
        ))
    }
}

impl_lift_for_struct!(TsInterfaceBody, [span, body]);

impl_lift_for_struct!(TsTypeAnn, [span, type_ann]);

impl_lift_for_struct!(TsTypeParamDecl, [span, params]);

impl_lift_for_struct!(
    TsTypeParam,
    [span, name, is_in, is_out, is_const, constraint, default]
);

impl_lift_for_struct!(TsExprWithTypeArgs, [span, expr, type_args]);
