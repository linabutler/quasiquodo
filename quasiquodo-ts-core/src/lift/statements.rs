use swc_ecma_ast::*;
use syn::parse_quote;

use crate::{context::Context, input::VarType};

use super::{CodeFragment, Lift, impl_lift_for_newtype_enum, impl_lift_for_struct, lift_variants};

/// Custom implementation to splice `Stmt` and `Decl` variables.
///
/// For `Stmt` variables, the preprocessor inserts stand-ins that parse as
/// `ExprStmt(Ident)`. This custom implementation detects and replaces
/// those stand-ins with the bound variables.
///
/// For `Decl` stand-ins, the preprocessor inserts `var __tsq_N__` stand-ins
/// that parse as `Stmt::Decl(VarDecl(...))`. [`Decl::lift`] returns bare
/// `Decl` splices, so this implementation wraps those splices in `Stmt::Decl`.
impl Lift for Stmt {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        // Handle `Stmt` splices.
        if let Stmt::Expr(ExprStmt { expr, .. }) = self
            && let Expr::Ident(ident) = &**expr
            && let Some(var) = context.stand_in(&ident.sym)
            && matches!(var.ty.inner(), VarType::Stmt)
        {
            let var_ident = &var.ident;
            return Ok(match &var.ty {
                VarType::Vec(_) | VarType::Option(_) => {
                    CodeFragment::Splice(parse_quote!(#var_ident.iter().cloned()))
                }
                _ => CodeFragment::Single(parse_quote!(
                    #root::swc::ecma_ast::Stmt::from(#var_ident.clone())
                )),
            });
        }

        // Handle `Decl` splices.
        if let Stmt::Decl(inner) = self {
            return Ok(match inner.lift(context)? {
                CodeFragment::Single(expr) => CodeFragment::Single(parse_quote!(
                    #root::swc::ecma_ast::Stmt::Decl(#expr)
                )),
                CodeFragment::Splice(expr) => CodeFragment::Splice(parse_quote!(
                    (#expr).map(#root::swc::ecma_ast::Stmt::Decl)
                )),
            });
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
