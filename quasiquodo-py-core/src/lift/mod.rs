use proc_macro2::Span;
use ruff_python_ast::*;

use super::context::Context;

mod expressions;
mod operators;
mod parameters;
mod primitives;
mod statements;

/// Turns a [`ruff_python_ast`] node into Rust code that, when compiled,
/// evaluates to that node.
pub(crate) trait Lift {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment>;
}

/// The result of converting a [`ruff_python_ast`] node to Rust code.
pub(crate) enum CodeFragment {
    /// A single expression that constructs one AST node.
    Single(syn::Expr),

    /// A splice expression that produces an iterator of nodes,
    /// propagated upward to the closest iterable position.
    Splice(syn::Expr),
}

/// Extracts the [`syn::Expr`] from a [`CodeFragment::Single`], or
/// propagates a [`CodeFragment::Splice`] upward via an early return.
macro_rules! unsplice {
    ($fragment:expr) => {
        match $fragment {
            CodeFragment::Single(expr) => expr,
            CodeFragment::Splice(expr) => return Ok(CodeFragment::Splice(expr)),
        }
    };
}

/// Implements [`Lift`] for an AST struct, calling [`Lift::lift`] on each field.
/// If any field lifts to a [`CodeFragment::Splice`], returns that splice for
/// propagating upward.
macro_rules! impl_lift_for_struct {
    ($name:ident, [ $($field:ident),* ]) => {
        impl Lift for $name {
            fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
                let Self { $($field,)* } = self;
                $(
                    let $field = crate::lift::unsplice!(Lift::lift($field, context)?);
                )*
                let root = context.root();
                Ok(CodeFragment::Single(parse_quote!(#root::ruff::python_ast::$name {
                    $($field: #$field,)*
                })))
            }
        }
    };
}

macro_rules! lift_variants {
    ($v:expr, $context:expr, $name:ident, [ $($variant:ident),* ]) => {
        match $v {
            $($name::$variant(inner) => {
                let expr = crate::lift::unsplice!(Lift::lift(inner, $context)?);
                let root = $context.root();
                Ok(CodeFragment::Single(parse_quote!(
                    #root::ruff::python_ast::$name::$variant(#expr)
                )))
            },)*
        }
    };
}

/// Implements [`Lift`] for an enum whose variants are all newtype variants,
/// calling [`Lift::lift`] on each variant. If the variant lifts to a
/// [`CodeFragment::Splice`], returns that splice for propagating upward.
macro_rules! impl_lift_for_newtype_enum {
    ($name:ident, [ $($variant:ident),* ]) => {
        impl Lift for $name {
            fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
                lift_variants!(self, context, $name, [ $($variant),* ])
            }
        }
    };
}

/// Implements [`Lift`] for an enum whose variants are all unit variants,
/// mapping each variant to its name as a [`syn::Ident`].
macro_rules! impl_lift_for_unit_enum {
    ($name:ident, [ $($variant:ident),* ]) => {
        impl Lift for $name {
            fn lift(&self, context: &Context) -> syn::Result<CodeFragment> {
                let variant = match self {
                    $($name::$variant => stringify!($variant),)*
                };
                let variant_ident = syn::Ident::new(variant, Span::call_site());
                let root = context.root();
                Ok(CodeFragment::Single(
                    parse_quote!(#root::ruff::python_ast::$name::#variant_ident),
                ))
            }
        }
    };
}

pub(crate) use impl_lift_for_newtype_enum;
pub(crate) use impl_lift_for_struct;
pub(crate) use impl_lift_for_unit_enum;
pub(crate) use lift_variants;
pub(crate) use unsplice;

/// Implements [`Lift`] for one or more unsupported node types, returning a
/// compile-time error that names the unsupported type.
macro_rules! impl_lift_unsupported {
    ($($name:ident),* $(,)?) => {
        $(
            impl Lift for $name {
                fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
                    Err(syn::Error::new(
                        Span::call_site(),
                        concat!("`", stringify!($name), "` can't be turned into Rust code"),
                    ))
                }
            }
        )*
    };
}

impl_lift_unsupported!(ExprIpyEscapeCommand, StmtIpyEscapeCommand);
