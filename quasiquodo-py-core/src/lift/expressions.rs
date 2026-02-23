use ruff_python_ast::*;
use syn::parse_quote;

use crate::{
    context::Context,
    input::{NumVarType, VarType},
};

use super::{CodeFragment, Lift, impl_lift_for_struct, lift_variants};

/// Custom implementation that replaces stand-ins with real bound variables.
impl Lift for Expr {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
        let root = context.root();
        if let Expr::Name(name) = self
            && let Some(var) = context.stand_in(name.id.as_str())
        {
            let var_ident = &var.ident;
            match &var.ty {
                VarType::Box(inner) if **inner == VarType::Expr => {
                    return Ok(CodeFragment::Single(parse_quote!(
                        #root::ruff::python_ast::Expr::from(*#var_ident.clone())
                    )));
                }
                VarType::Expr => {
                    return Ok(CodeFragment::Single(parse_quote!(
                        #root::ruff::python_ast::Expr::from(#var_ident.clone())
                    )));
                }
                ty if ty.is_str() => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(#root::ruff::python_ast::Expr::StringLiteral(
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
                        )),
                    ));
                }
                VarType::Num(NumVarType::F64) => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(#root::ruff::python_ast::Expr::NumberLiteral(
                            #root::ruff::python_ast::ExprNumberLiteral {
                                node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                                range: #root::ruff::text_size::TextRange::default(),
                                value: #root::ruff::python_ast::Number::Float(#var_ident),
                            }
                        )),
                    ));
                }
                VarType::Num(
                    NumVarType::U8 | NumVarType::U16 | NumVarType::U32 | NumVarType::U64,
                ) => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(#root::ruff::python_ast::Expr::NumberLiteral(
                            #root::ruff::python_ast::ExprNumberLiteral {
                                node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                                range: #root::ruff::text_size::TextRange::default(),
                                value: #root::ruff::python_ast::Number::Int(
                                    #root::ruff::python_ast::Int::from(#var_ident),
                                ),
                            }
                        )),
                    ));
                }
                VarType::Bool => {
                    return Ok(CodeFragment::Single(
                        parse_quote!(#root::ruff::python_ast::Expr::BooleanLiteral(
                            #root::ruff::python_ast::ExprBooleanLiteral {
                                node_index: #root::ruff::python_ast::AtomicNodeIndex::NONE,
                                range: #root::ruff::text_size::TextRange::default(),
                                value: #var_ident,
                            }
                        )),
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
                BoolOp,
                Named,
                BinOp,
                UnaryOp,
                Lambda,
                If,
                Dict,
                Set,
                ListComp,
                SetComp,
                DictComp,
                Generator,
                Await,
                Yield,
                YieldFrom,
                Compare,
                Call,
                FString,
                TString,
                StringLiteral,
                BytesLiteral,
                NumberLiteral,
                BooleanLiteral,
                NoneLiteral,
                EllipsisLiteral,
                Attribute,
                Subscript,
                Starred,
                Name,
                List,
                Tuple,
                Slice,
                IpyEscapeCommand
            ]
        )
    }
}

impl_lift_for_struct!(ExprBoolOp, [node_index, range, op, values]);

impl_lift_for_struct!(ExprNamed, [node_index, range, target, value]);

impl_lift_for_struct!(ExprBinOp, [node_index, range, left, op, right]);

impl_lift_for_struct!(ExprUnaryOp, [node_index, range, op, operand]);

impl_lift_for_struct!(ExprLambda, [node_index, range, parameters, body]);

impl_lift_for_struct!(ExprIf, [node_index, range, test, body, orelse]);

impl_lift_for_struct!(ExprDict, [node_index, range, items]);

impl_lift_for_struct!(DictItem, [key, value]);

impl_lift_for_struct!(ExprSet, [node_index, range, elts]);

impl_lift_for_struct!(ExprListComp, [node_index, range, elt, generators]);

impl_lift_for_struct!(ExprSetComp, [node_index, range, elt, generators]);

impl_lift_for_struct!(ExprDictComp, [node_index, range, key, value, generators]);

impl_lift_for_struct!(
    ExprGenerator,
    [node_index, range, elt, generators, parenthesized]
);

impl_lift_for_struct!(ExprAwait, [node_index, range, value]);

impl_lift_for_struct!(ExprYield, [node_index, range, value]);

impl_lift_for_struct!(ExprYieldFrom, [node_index, range, value]);

impl_lift_for_struct!(ExprCompare, [node_index, range, left, ops, comparators]);

impl_lift_for_struct!(ExprCall, [node_index, range, func, arguments]);

impl_lift_for_struct!(ExprStringLiteral, [node_index, range, value]);

impl_lift_for_struct!(ExprFString, [node_index, range, value]);

impl_lift_for_struct!(ExprTString, [node_index, range, value]);

impl_lift_for_struct!(ExprBytesLiteral, [node_index, range, value]);

impl_lift_for_struct!(ExprNumberLiteral, [node_index, range, value]);

impl_lift_for_struct!(ExprBooleanLiteral, [node_index, range, value]);

impl_lift_for_struct!(ExprNoneLiteral, [node_index, range]);

impl_lift_for_struct!(ExprEllipsisLiteral, [node_index, range]);

impl_lift_for_struct!(ExprAttribute, [node_index, range, value, attr, ctx]);

impl_lift_for_struct!(ExprSubscript, [node_index, range, value, slice, ctx]);

impl_lift_for_struct!(ExprStarred, [node_index, range, value, ctx]);

impl_lift_for_struct!(ExprList, [node_index, range, elts, ctx]);

impl_lift_for_struct!(ExprTuple, [node_index, range, elts, ctx, parenthesized]);

impl_lift_for_struct!(ExprSlice, [node_index, range, lower, upper, step]);
