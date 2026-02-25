use proc_macro2::Span;
use swc_ecma_ast::*;
use syn::parse_quote;

use crate::{context::Context, input::VarType};

use super::{
    CodeFragment, Lift, impl_lift_for_newtype_enum, impl_lift_for_struct, impl_lift_for_unit_enum,
    lift_variants, unsplice,
};

/// Custom implementation to splice `Expr` and literal variables.
impl Lift for Expr {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        if let Expr::Ident(ident) = self
            && let Some(var) = context.stand_in(&ident.sym)
        {
            let var_ident = var.to_tokens();
            let span_expr = context.span();
            match &var.ty {
                VarType::Box(inner) if **inner == VarType::Expr => {
                    return Ok(CodeFragment::Single(parse_quote!(*#var_ident)));
                }
                VarType::Expr => {
                    return Ok(CodeFragment::Single(parse_quote!(#var_ident)));
                }
                VarType::LitStr => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(::quasiquodo::ts::swc::ecma_ast::Expr::Lit(::quasiquodo::ts::swc::ecma_ast::Lit::Str(
                            ::quasiquodo::ts::swc::ecma_ast::Str {
                                span: #span_expr,
                                value: (#var_ident).into(),
                                raw: None,
                            }
                        ))),
                    ));
                }
                VarType::LitNum => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(::quasiquodo::ts::swc::ecma_ast::Expr::Lit(::quasiquodo::ts::swc::ecma_ast::Lit::Num(
                            ::quasiquodo::ts::swc::ecma_ast::Number {
                                span: #span_expr,
                                value: #var_ident,
                                raw: None,
                            }
                        ))),
                    ));
                }
                VarType::LitBool => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(::quasiquodo::ts::swc::ecma_ast::Expr::Lit(::quasiquodo::ts::swc::ecma_ast::Lit::Bool(
                            ::quasiquodo::ts::swc::ecma_ast::Bool {
                                span: #span_expr,
                                value: #var_ident,
                            }
                        ))),
                    ));
                }
                _ => (),
            }
        }
        lift_variants!(
            self,
            context,
            Expr,
            [
                This,
                Array,
                Object,
                Fn,
                Unary,
                Update,
                Bin,
                Assign,
                Member,
                SuperProp,
                Cond,
                Call,
                New,
                Seq,
                Ident,
                Lit,
                Tpl,
                TaggedTpl,
                Arrow,
                Class,
                Yield,
                MetaProp,
                Await,
                Paren,
                TsTypeAssertion,
                TsConstAssertion,
                TsNonNull,
                TsAs,
                TsInstantiation,
                TsSatisfies,
                PrivateName,
                OptChain,
                Invalid,
                JSXMember,
                JSXNamespacedName,
                JSXEmpty,
                JSXElement,
                JSXFragment
            ]
        )
    }
}

impl_lift_for_struct!(ThisExpr, [span]);

impl_lift_for_struct!(ArrayLit, [span, elems]);

impl_lift_for_struct!(FnExpr, [ident, function]);

impl_lift_for_struct!(UnaryExpr, [span, op, arg]);

impl_lift_for_struct!(UpdateExpr, [span, op, prefix, arg]);

impl_lift_for_struct!(BinExpr, [span, op, left, right]);

impl_lift_for_struct!(AssignExpr, [span, op, left, right]);

impl_lift_for_struct!(MemberExpr, [span, obj, prop]);

impl_lift_for_struct!(SuperPropExpr, [span, obj, prop]);

impl_lift_for_struct!(Super, [span]);

impl_lift_for_struct!(CondExpr, [span, test, cons, alt]);

impl_lift_for_struct!(CallExpr, [span, ctxt, callee, args, type_args]);

impl_lift_for_struct!(NewExpr, [span, ctxt, callee, args, type_args]);

impl_lift_for_struct!(SeqExpr, [span, exprs]);

impl_lift_for_struct!(ExprOrSpread, [spread, expr]);

impl_lift_for_struct!(Tpl, [span, exprs, quasis]);

impl_lift_for_struct!(TaggedTpl, [span, ctxt, tag, type_params, tpl]);

impl_lift_for_struct!(
    ArrowExpr,
    [
        span,
        ctxt,
        params,
        body,
        is_async,
        is_generator,
        type_params,
        return_type
    ]
);

impl_lift_for_struct!(ClassExpr, [ident, class]);

impl_lift_for_struct!(YieldExpr, [span, arg, delegate]);

impl_lift_for_struct!(MetaPropExpr, [span, kind]);

impl_lift_for_struct!(AwaitExpr, [span, arg]);

impl_lift_for_struct!(ParenExpr, [span, expr]);

impl_lift_for_struct!(OptChainExpr, [span, optional, base]);

impl_lift_for_struct!(OptCall, [span, ctxt, callee, args, type_args]);

impl_lift_for_struct!(Import, [span, phase]);

impl_lift_for_struct!(TsTypeAssertion, [span, expr, type_ann]);

impl_lift_for_struct!(TsConstAssertion, [span, expr]);

impl_lift_for_struct!(TsNonNullExpr, [span, expr]);

impl_lift_for_struct!(TsAsExpr, [span, expr, type_ann]);

impl_lift_for_struct!(TsInstantiation, [span, expr, type_args]);

impl_lift_for_struct!(TsSatisfiesExpr, [span, expr, type_ann]);

/// Custom implementation to rewrite computed properties with `LitStr` variables
/// as bare identifier properties where valid.
impl Lift for MemberProp {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let MemberProp::Computed(computed) = self else {
            return lift_variants!(self, context, MemberProp, [Ident, PrivateName, Computed]);
        };
        // Check if this is a stand-in for a `LitStr` variable
        // inserted during preprocessing.
        let var = if let Expr::Lit(Lit::Str(s)) = &*computed.expr
            && let Some(value) = s.value.as_str()
            && let Some(var) = context.stand_in(value)
            && matches!(var.ty, VarType::LitStr)
        {
            Some(var)
        } else if let Expr::Ident(ident) = &*computed.expr
            && let Some(var) = context.stand_in(&ident.sym)
            && matches!(var.ty, VarType::LitStr)
        {
            Some(var)
        } else {
            None
        };
        match var {
            // Generate conditional code that chooses between `Ident` and
            // `Computed`, depending on whether the string variable's value
            // would be a valid identifier.
            Some(var) => {
                let var_ident = var.to_tokens();
                let span_expr = context.span();
                Ok(CodeFragment::Single(parse_quote!({
                    let name = #var_ident;
                    if ::quasiquodo::ts::swc::ecma_utils::is_valid_prop_ident(name) {
                        ::quasiquodo::ts::swc::ecma_ast::MemberProp::Ident(::quasiquodo::ts::swc::ecma_ast::IdentName {
                            span: #span_expr,
                            sym: name.into(),
                        })
                    } else {
                        ::quasiquodo::ts::swc::ecma_ast::MemberProp::Computed(::quasiquodo::ts::swc::ecma_ast::ComputedPropName {
                            span: #span_expr,
                            expr: Box::new(::quasiquodo::ts::swc::ecma_ast::Expr::Lit(::quasiquodo::ts::swc::ecma_ast::Lit::Str(
                                ::quasiquodo::ts::swc::ecma_ast::Str {
                                    span: #span_expr,
                                    value: name.into(),
                                    raw: None,
                                }
                            ))),
                        })
                    }
                })))
            }
            // In all other cases, emit the computed property verbatim.
            None => {
                let val = unsplice!(Lift::lift(computed, context)?);
                Ok(CodeFragment::Single(
                    parse_quote!(::quasiquodo::ts::swc::ecma_ast::MemberProp::Computed(#val)),
                ))
            }
        }
    }
}

impl_lift_for_newtype_enum!(SuperProp, [Ident, Computed]);

impl_lift_for_newtype_enum!(OptChainBase, [Member, Call]);

impl_lift_for_newtype_enum!(Callee, [Super, Import, Expr]);

impl_lift_for_newtype_enum!(BlockStmtOrExpr, [BlockStmt, Expr]);

impl_lift_for_newtype_enum!(AssignTarget, [Simple, Pat]);

impl_lift_for_newtype_enum!(
    SimpleAssignTarget,
    [
        Ident,
        Member,
        SuperProp,
        Paren,
        OptChain,
        TsAs,
        TsSatisfies,
        TsNonNull,
        TsTypeAssertion,
        TsInstantiation,
        Invalid
    ]
);

impl_lift_for_newtype_enum!(AssignTargetPat, [Array, Object, Invalid]);

impl_lift_for_unit_enum!(UnaryOp, [Minus, Plus, Bang, Tilde, TypeOf, Void, Delete]);

impl_lift_for_unit_enum!(UpdateOp, [PlusPlus, MinusMinus]);

impl_lift_for_unit_enum!(
    BinaryOp,
    [
        EqEq,
        NotEq,
        EqEqEq,
        NotEqEq,
        Lt,
        LtEq,
        Gt,
        GtEq,
        LShift,
        RShift,
        ZeroFillRShift,
        Add,
        Sub,
        Mul,
        Div,
        Mod,
        BitOr,
        BitXor,
        BitAnd,
        LogicalOr,
        LogicalAnd,
        In,
        InstanceOf,
        Exp,
        NullishCoalescing
    ]
);

impl_lift_for_unit_enum!(
    AssignOp,
    [
        Assign,
        AddAssign,
        SubAssign,
        MulAssign,
        DivAssign,
        ModAssign,
        LShiftAssign,
        RShiftAssign,
        ZeroFillRShiftAssign,
        BitOrAssign,
        BitXorAssign,
        BitAndAssign,
        ExpAssign,
        AndAssign,
        OrAssign,
        NullishAssign
    ]
);

impl_lift_for_unit_enum!(MetaPropKind, [NewTarget, ImportMeta]);
