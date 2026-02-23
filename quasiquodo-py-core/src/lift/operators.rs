use proc_macro2::Span;
use ruff_python_ast::*;
use syn::parse_quote;

use crate::context::Context;

use super::{CodeFragment, Lift, impl_lift_for_unit_enum};

impl_lift_for_unit_enum!(
    Operator,
    [
        Add, Sub, Mult, MatMult, Div, Mod, Pow, LShift, RShift, BitOr, BitXor, BitAnd, FloorDiv
    ]
);

impl_lift_for_unit_enum!(UnaryOp, [Invert, Not, UAdd, USub]);

impl_lift_for_unit_enum!(BoolOp, [And, Or]);

impl_lift_for_unit_enum!(CmpOp, [Eq, NotEq, Lt, LtE, Gt, GtE, Is, IsNot, In, NotIn]);
