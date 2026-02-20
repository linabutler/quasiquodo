use swc_ecma_ast::*;
use syn::parse_quote;

use crate::{context::Context, input::VarType};

use super::{CodeFragment, Lift, impl_lift_for_newtype_enum, impl_lift_for_struct, lift_variants};

/// Custom implementation that intercepts `Decl` and `Stmt` placeholders
/// in expression-statement position. `Decl` variables are wrapped in
/// `Stmt::Decl`; `Stmt` variables are passed through directly.
impl Lift for Stmt {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        // Check for `Decl` or `Stmt` cross-type placeholders in
        // expression-statement positions. Without this check,
        // `Ident`-level splice would propagate upward, which would
        // fail if `Stmt` is the top-level output type, with no
        // `Vec` above to catch the splice.
        if let Stmt::Expr(ExprStmt { expr, .. }) = self
            && let Expr::Ident(ident) = &**expr
            && let Some(var) = context.placeholder(&ident.sym)
        {
            let var_ident = var.to_tokens();
            match &var.ty {
                VarType::Decl => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(::quasiquodo::ts::swc::ecma_ast::Stmt::Decl(#var_ident)),
                    ));
                }
                VarType::Stmt => {
                    return Ok(CodeFragment::Single(parse_quote!(#var_ident)));
                }
                VarType::Vec(inner) | VarType::Option(inner)
                    if matches!(**inner, VarType::Decl) =>
                {
                    return Ok(CodeFragment::Splice(parse_quote!(
                        #var_ident.into_iter().map(::quasiquodo::ts::swc::ecma_ast::Stmt::Decl)
                    )));
                }
                VarType::Vec(inner) | VarType::Option(inner)
                    if matches!(**inner, VarType::Stmt) =>
                {
                    return Ok(CodeFragment::Splice(parse_quote!(#var_ident.into_iter())));
                }
                _ => (),
            }
        }
        lift_variants!(
            self,
            context,
            Stmt,
            [
                Block, Empty, Debugger, With, Return, Labeled, Break, Continue, If, Switch, Throw,
                Try, While, DoWhile, For, ForIn, ForOf, Decl, Expr
            ]
        )
    }
}

impl_lift_for_struct!(ExprStmt, [span, expr]);

impl_lift_for_struct!(BlockStmt, [span, ctxt, stmts]);

impl_lift_for_struct!(DebuggerStmt, [span]);

impl_lift_for_struct!(WithStmt, [span, obj, body]);

impl_lift_for_struct!(ReturnStmt, [span, arg]);

impl_lift_for_struct!(LabeledStmt, [span, label, body]);

impl_lift_for_struct!(BreakStmt, [span, label]);

impl_lift_for_struct!(ContinueStmt, [span, label]);

impl_lift_for_struct!(IfStmt, [span, test, cons, alt]);

impl_lift_for_struct!(SwitchStmt, [span, discriminant, cases]);

impl_lift_for_struct!(SwitchCase, [span, test, cons]);

impl_lift_for_struct!(ThrowStmt, [span, arg]);

impl_lift_for_struct!(TryStmt, [span, block, handler, finalizer]);

impl_lift_for_struct!(CatchClause, [span, param, body]);

impl_lift_for_struct!(WhileStmt, [span, test, body]);

impl_lift_for_struct!(DoWhileStmt, [span, test, body]);

impl_lift_for_struct!(ForStmt, [span, init, test, update, body]);

impl_lift_for_struct!(ForInStmt, [span, left, right, body]);

impl_lift_for_struct!(ForOfStmt, [span, is_await, left, right, body]);

impl_lift_for_newtype_enum!(VarDeclOrExpr, [VarDecl, Expr]);

impl_lift_for_newtype_enum!(ForHead, [VarDecl, UsingDecl, Pat]);
