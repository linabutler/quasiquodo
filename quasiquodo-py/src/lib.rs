#[doc(hidden)]
pub use quasiquodo_py_macros::py_quote_with_root;

/// Parses a Python source string into a syntax tree at compile time,
/// and expands to Rust code that constructs the same syntax tree
/// in your program.
///
/// The output kind after `as` determines how the source is parsed,
/// and what [`ruff_python_ast`][self::ruff::python_ast] type is returned.
///
/// ```
/// use quasiquodo_py::py_quote;
///
/// let expr = py_quote!("x + 1" as Expr);
/// ```
///
/// `#{name}` placeholders splice Rust values into the syntax tree.
/// Each placeholder is bound to a variable declared after the output
/// kind, as `name: Type = value`:
///
/// ```
/// # use quasiquodo_py::py_quote;
/// # use quasiquodo_py::ruff::python_ast::Identifier;
/// # fn f(my_name: Identifier) {
/// let call = py_quote!("#{name}()" as Expr, name: Identifier = my_name);
/// # }
/// ```
///
/// [`Vec<T>`] variables splice into list positions; [`Option<T>`]
/// variables conditionally include other nodes.
///
/// All range fields in the generated AST are set to
/// [`TextRange::default()`][self::ruff::text_size::TextRange] and all
/// node indices to `AtomicNodeIndex::NONE`.
#[macro_export]
macro_rules! py_quote {
    ($($tt:tt)*) => {
        $crate::py_quote_with_root!($crate; $($tt)*)
    };
}

pub mod ruff {
    pub use ruff_python_ast as python_ast;
    pub use ruff_python_stdlib as python_stdlib;
    pub use ruff_text_size as text_size;
}
