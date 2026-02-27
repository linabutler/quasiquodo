#[cfg(feature = "ts")]
#[doc(hidden)]
pub use quasiquodo_ts::ts_quote_with_root;

/// Parses a TypeScript source string into a syntax tree at compile time,
/// and expands to Rust code that constructs the same syntax tree
/// in your program.
///
/// The output kind after `as` determines how the source is parsed,
/// and what [`swc_ecma_ast`][self::ts::swc::ecma_ast] type is returned.
///
/// ```
/// use quasiquodo::ts_quote;
///
/// let ty = ts_quote!("string | null" as TsType);
/// ```
///
/// `#{name}` placeholders splice Rust values into the syntax tree.
/// Each placeholder is bound to a variable declared after the output
/// kind, as `name: Type = value`:
///
/// ```
/// # use quasiquodo::ts_quote;
/// # use quasiquodo::ts::swc::ecma_ast::Expr;
/// # fn f(my_expr: Expr) {
/// let call = ts_quote!("foo(#{arg})" as Expr, arg: Expr = my_expr);
/// # }
/// ```
///
/// [`Vec<T>`] variables splice into list positions; [`Option<T>`]
/// variables conditionally include other nodes.
///
/// Pass a [`comments [= Comments]`][self::ts::Comments] argument to collect
/// JSDoc comments for code generation, and a `span = expr` argument to
/// use a custom [`Span`][self::ts::swc::common::Span]. Both arguments
/// are optional: all comments are discarded by default, and spans
/// default to [`DUMMY_SP`][self::ts::swc::common::DUMMY_SP].
#[cfg(feature = "ts")]
#[macro_export]
macro_rules! ts_quote {
    ($($tt:tt)*) => {
        $crate::ts_quote_with_root!($crate::ts; $($tt)*)
    };
}

/// Additional types and re-exports for `ts_quote!`.
#[cfg(feature = "ts")]
pub mod ts {
    #[doc(inline)]
    pub use quasiquodo_ts::{Comments, JsDoc, num_bigint, swc};
}
