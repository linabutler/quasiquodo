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
        {
            let var_ident = &var.ident;
            let root = context.root();
            match &var.ty {
                VarType::Stmt => {
                    return Ok(CodeFragment::Single(parse_quote!(
                        #root::ruff::python_ast::Stmt::from(#var_ident.clone())
                    )));
                }
                VarType::Suite | VarType::Vec(_) | VarType::Option(_)
                    if matches!(var.ty.inner(), VarType::Stmt | VarType::Suite) =>
                {
                    return Ok(CodeFragment::Splice(
                        parse_quote!(#var_ident.iter().cloned()),
                    ));
                }
                ty if ty.is_str() => {
                    return Ok(CodeFragment::Single(parse_quote!(
                        #root::ruff::python_ast::Stmt::from(
                            #root::ruff::python_ast::StmtExpr {
                                node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                                range: #root::ruff::text_size::TextRange::default(),
                                value: Box::new(
                                    #root::ruff::python_ast::Expr::StringLiteral(
                                        #root::ruff::python_ast::ExprStringLiteral {
                                            node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                                            range: #root::ruff::text_size::TextRange::default(),
                                            value: #root::ruff::python_ast::StringLiteralValue::single(
                                                #root::ruff::python_ast::StringLiteral {
                                                    range: #root::ruff::text_size::TextRange::default(),
                                                    node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                                                    value: Box::from(#var_ident.clone()),
                                                    flags: #root::ruff::python_ast::StringLiteralFlags::empty(),
                                                }
                                            ),
                                        }
                                    )
                                ),
                            }
                        )
                    )));
                }
                VarType::Option(inner) | VarType::Vec(inner) if inner.is_str() => {
                    return Ok(CodeFragment::Splice(
                        parse_quote!(#var_ident.iter().map(|text| {
                            #root::ruff::python_ast::Stmt::from(
                                #root::ruff::python_ast::StmtExpr {
                                    node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                                    range: #root::ruff::text_size::TextRange::default(),
                                    value: Box::new(
                                        #root::ruff::python_ast::Expr::StringLiteral(
                                            #root::ruff::python_ast::ExprStringLiteral {
                                                node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                                                range: #root::ruff::text_size::TextRange::default(),
                                                value: #root::ruff::python_ast::StringLiteralValue::single(
                                                    #root::ruff::python_ast::StringLiteral {
                                                        range: #root::ruff::text_size::TextRange::default(),
                                                        node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                                                        value: Box::from(&**text),
                                                        flags: #root::ruff::python_ast::StringLiteralFlags::empty(),
                                                    }
                                                ),
                                            }
                                        )
                                    ),
                                }
                            )
                        })),
                    ));
                }
                _ => (),
            }
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
