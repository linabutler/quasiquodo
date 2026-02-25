use proc_macro2::Span;
use swc_ecma_ast::*;
use syn::parse_quote;

use super::context::Context;

mod declarations;
mod expressions;
mod modules;
mod primitives;
mod statements;
mod types;

/// Turns an [`swc_ecma_ast`] node into Rust code that, when compiled,
/// evaluates to that node.
pub(crate) trait Lift {
    fn lift(&self, context: &Context) -> syn::Result<CodeFragment>;
}

/// The result of converting an [`swc_ecma_ast`] node to Rust code.
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
                Ok(CodeFragment::Single(parse_quote!(::quasiquodo::ts::swc::ecma_ast::$name {
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
                Ok(CodeFragment::Single(parse_quote!(
                    ::quasiquodo::ts::swc::ecma_ast::$name::$variant(#expr)
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
            fn lift(&self, _: &Context) -> syn::Result<CodeFragment> {
                let variant = match self {
                    $($name::$variant => stringify!($variant),)*
                };
                let variant_ident = syn::Ident::new(variant, Span::call_site());
                Ok(CodeFragment::Single(
                    parse_quote!(::quasiquodo::ts::swc::ecma_ast::$name::#variant_ident),
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

/// Enables cross-type splicing of `Vec<Ident>` and `Option<Ident>`
/// variables into container types that can wrap a bare identifier.
pub(crate) trait SpliceIdent {
    /// If this node is a stand-in for a `Vec<Ident>` or `Option<Ident>`
    /// variable, returns an iterator expression that maps each `Ident`
    /// into this container type.
    ///
    /// Returns `None` if this node isn't a stand-in.
    fn splice_ident(&self, context: &Context) -> Option<syn::Expr>;
}

/// Lifts a slice of AST nodes into a `Vec` constructor expression,
/// allowing `Vec<Ident>` variables to splice into positions that expect
/// a different element type.
pub(crate) fn splice_idents<T>(items: &[T], context: &Context) -> syn::Result<syn::Expr>
where
    T: Lift + SpliceIdent,
{
    let stmts = items
        .iter()
        .map(|item| {
            let expr: syn::Expr = match item.splice_ident(context) {
                Some(expr) => parse_quote!(items.extend(#expr)),
                None => match item.lift(context)? {
                    CodeFragment::Single(expr) => parse_quote!(items.push(#expr)),
                    CodeFragment::Splice(expr) => {
                        parse_quote!(items.extend((#expr).map(Into::into)))
                    }
                },
            };
            Ok(expr)
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let capacity = stmts.len();
    Ok(parse_quote! {{
        let mut items = Vec::with_capacity(#capacity);
        #(#stmts;)*
        items
    }})
}

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

impl_lift_unsupported!(
    JSXText,
    JSXMemberExpr,
    JSXNamespacedName,
    JSXEmptyExpr,
    JSXElement,
    JSXFragment,
);
