use std::{cell::Cell, ops::Deref};

use self::swc::{
    atoms::Atom,
    common::{
        BytePos, DUMMY_SP, Span,
        comments::{Comment, CommentKind, Comments as SwcComments, SingleThreadedComments},
    },
};

pub mod swc {
    pub use swc_atoms as atoms;
    pub use swc_common as common;
    pub use swc_ecma_ast as ecma_ast;
    pub use swc_ecma_utils as ecma_utils;
}

pub use num_bigint;

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
            // [`BytePos`] reserves offsets larger than `u32::MAX - 2^16`
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
