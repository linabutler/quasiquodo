use ruff_python_ast::*;
use syn::parse_quote;

use crate::{context::Context, input::VarType};

use super::{CodeFragment, Lift, impl_lift_for_newtype_enum, impl_lift_for_struct, lift_variants};

/// Custom implementation with stand-in replacement.
///
/// The preprocessor emits stand-ins that parse as `StmtExpr(ExprName)`.
/// This implementation detects and substitutes those stand-ins with
/// the real bound variables.
impl Lift for Stmt {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        if let Stmt::Expr(StmtExpr { value, .. }) = self
            && let Expr::Name(name) = &**value
            && let Some(var) = context.stand_in(name.id.as_str())
            && matches!(var.ty.inner(), VarType::Stmt)
        {
            let var_ident = &var.ident;
            return Ok(match &var.ty {
                VarType::Vec(_) | VarType::Option(_) => {
                    CodeFragment::Splice(parse_quote!(#var_ident.iter().cloned()))
                }
                _ => {
                    let root = context.root();
                    CodeFragment::Single(parse_quote!(
                        #root::ruff::python_ast::Stmt::from(#var_ident.clone())
                    ))
                }
            });
        }

        lift_variants!(
            self,
            context,
            Stmt,
            [
                FunctionDef,
                ClassDef,
                Return,
                Delete,
                TypeAlias,
                Assign,
                AugAssign,
                AnnAssign,
                For,
                While,
                If,
                With,
                Match,
                Raise,
                Try,
                Assert,
                Import,
                ImportFrom,
                Global,
                Nonlocal,
                Expr,
                Pass,
                Break,
                Continue,
                IpyEscapeCommand
            ]
        )
    }
}

impl_lift_for_struct!(
    StmtFunctionDef,
    [
        node_index,
        range,
        is_async,
        decorator_list,
        name,
        type_params,
        parameters,
        returns,
        body
    ]
);

impl_lift_for_struct!(
    StmtClassDef,
    [
        node_index,
        range,
        decorator_list,
        name,
        type_params,
        arguments,
        body
    ]
);

impl_lift_for_struct!(StmtReturn, [node_index, range, value]);

impl_lift_for_struct!(StmtDelete, [node_index, range, targets]);

impl_lift_for_struct!(StmtTypeAlias, [node_index, range, name, type_params, value]);

impl_lift_for_struct!(StmtAssign, [node_index, range, targets, value]);

impl_lift_for_struct!(StmtAugAssign, [node_index, range, target, op, value]);

impl_lift_for_struct!(
    StmtAnnAssign,
    [node_index, range, target, annotation, value, simple]
);

impl_lift_for_struct!(
    StmtFor,
    [node_index, range, is_async, target, iter, body, orelse]
);

impl_lift_for_struct!(StmtWhile, [node_index, range, test, body, orelse]);

impl_lift_for_struct!(StmtIf, [node_index, range, test, body, elif_else_clauses]);

impl_lift_for_struct!(StmtWith, [node_index, range, is_async, items, body]);

impl_lift_for_struct!(StmtMatch, [node_index, range, subject, cases]);

impl_lift_for_struct!(StmtRaise, [node_index, range, exc, cause]);

impl_lift_for_struct!(
    StmtTry,
    [
        node_index, range, body, handlers, orelse, finalbody, is_star
    ]
);

impl_lift_for_struct!(StmtAssert, [node_index, range, test, msg]);

impl_lift_for_struct!(StmtImport, [node_index, range, names]);

impl_lift_for_struct!(StmtImportFrom, [node_index, range, module, names, level]);

impl_lift_for_struct!(StmtGlobal, [node_index, range, names]);

impl_lift_for_struct!(StmtNonlocal, [node_index, range, names]);

impl_lift_for_struct!(StmtExpr, [node_index, range, value]);

impl_lift_for_struct!(StmtPass, [node_index, range]);

impl_lift_for_struct!(StmtBreak, [node_index, range]);

impl_lift_for_struct!(StmtContinue, [node_index, range]);

impl_lift_for_newtype_enum!(ExceptHandler, [ExceptHandler]);

impl_lift_for_struct!(
    ExceptHandlerExceptHandler,
    [range, node_index, type_, name, body]
);
