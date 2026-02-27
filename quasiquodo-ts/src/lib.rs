use std::{cell::Cell, ops::Deref};

use self::swc::{
    atoms::Atom,
    common::{
        BytePos, DUMMY_SP, Span,
        comments::{Comment, CommentKind, Comments as SwcComments, SingleThreadedComments},
    },
};

#[doc(hidden)]
pub use quasiquodo_ts_macros::ts_quote_with_root;

/// Parses a TypeScript source string into a syntax tree at compile time,
/// and expands to Rust code that constructs the same syntax tree
/// in your program.
///
/// The output kind after `as` determines how the source is parsed,
/// and what [`swc_ecma_ast`][self::swc::ecma_ast] type is returned.
///
/// ```
/// use quasiquodo_ts::ts_quote;
///
/// let ty = ts_quote!("string | null" as TsType);
/// ```
///
/// `#{name}` placeholders splice Rust values into the syntax tree.
/// Each placeholder is bound to a variable declared after the output
/// kind, as `name: Type = value`:
///
/// ```
/// # use quasiquodo_ts::ts_quote;
/// # use quasiquodo_ts::swc::ecma_ast::Expr;
/// # fn f(my_expr: Expr) {
/// let call = ts_quote!("foo(#{arg})" as Expr, arg: Expr = my_expr);
/// # }
/// ```
///
/// [`Vec<T>`] variables splice into list positions; [`Option<T>`]
/// variables conditionally include other nodes.
///
/// Pass a [`comments [= Comments]`][Comments] argument to collect
/// JSDoc comments for code generation, and a `span = expr` argument to
/// use a custom [`Span`][self::swc::common::Span]. Both arguments are
/// optional: all comments are discarded by default, and spans default to
/// [`DUMMY_SP`][self::swc::common::DUMMY_SP].
#[macro_export]
macro_rules! ts_quote {
    ($($tt:tt)*) => {
        $crate::ts_quote_with_root!($crate; $($tt)*)
    };
}

pub mod swc {
    pub use swc_atoms as atoms;
    pub use swc_common as common;
    pub use swc_ecma_ast as ecma_ast;
    pub use swc_ecma_utils as ecma_utils;
}

pub use num_bigint;

/// Collects JSDoc `/** ... */` comments from `ts_quote!`.
///
/// The collected comments can then be rendered by
/// [`swc_ecma_codegen`][codegen]:
///
/// ```
/// use quasiquodo_ts::{Comments, ts_quote};
/// # use swc_ecma_codegen::to_code_with_comments;
///
/// let comments = Comments::new();
/// let ast = ts_quote!(
///     comments,
///     "/** A pet. */ name: string" as TsTypeElement,
/// );
/// # let code = to_code_with_comments(Some(&*comments), &ast);
/// ```
///
/// [`Comments`] can be passed to any SWC function that expects
/// an implementation of [its `Comments` trait][SwcComments].
///
/// [codegen]: https://rustdoc.swc.rs/swc_ecma_codegen/index.html
#[derive(Debug)]
pub struct Comments {
    inner: SingleThreadedComments,
    offset: Cell<u32>,
}

impl Deref for Comments {
    type Target = dyn SwcComments + 'static;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Default for Comments {
    #[inline]
    fn default() -> Self {
        Self {
            inner: SingleThreadedComments::default(),
            // `BytePos` reserves offsets larger than `u32::MAX - 2^16`
            // for comments, but that's not exposed as a constant,
            // so we inline it here.
            offset: Cell::new(u32::MAX - (1 << 16)),
        }
    }
}

impl Comments {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates a unique span with a pre-formatted block comment.
    /// `text` is the raw content between `/*` and `*/`.
    pub fn span_with_comment(&self, text: impl Into<Atom>) -> Span {
        let text = text.into();
        let pos = self.offset.get();
        self.offset.set(pos + 1);
        let lo = BytePos(pos);

        if !text.is_empty() {
            self.inner.add_leading(
                lo,
                Comment {
                    kind: CommentKind::Block,
                    span: DUMMY_SP,
                    text,
                },
            );
        }

        Span::new(lo, lo)
    }
}

/// A pre-built JSDoc comment that can be interpolated into
/// a TypeScript syntax tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsDoc(Atom);

impl JsDoc {
    /// Creates a [`JsDoc`] from a user-facing text string.
    pub fn new(text: impl std::fmt::Display) -> Self {
        Self(text.to_string().into())
    }

    /// Returns the comment text (between `/*` and `*/`), excluding
    /// the leading `*` prefix that marks a JSDoc block comment.
    pub fn raw_text(&self) -> Atom {
        self.0.clone()
    }
}
