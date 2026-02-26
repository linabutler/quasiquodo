use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;

use proc_macro2::Span;
use swc_common::{
    BytePos,
    comments::{Comment, SingleThreadedComments, SingleThreadedCommentsMapInner},
};
use syn::parse_quote;

use super::input::{MacroInput, VarType, Variable};

/// Prepares bound variables: emits Rust `let` bindings
/// and builds the substitution [`Context`].
pub(crate) fn context(
    input: &MacroInput,
    stand_ins: HashMap<String, StandInData>,
    comments: SingleThreadedComments,
) -> syn::Result<(Vec<syn::Stmt>, Context)> {
    use std::collections::hash_map::Entry;

    let mut bindings = vec![];
    let mut vars = HashMap::new();

    for Variable { name, ty, value } in &input.variables {
        match vars.entry(VarName(name.to_string())) {
            Entry::Occupied(entry) => {
                return Err(syn::Error::new(
                    name.span(),
                    format!("duplicate variable name `{}`", entry.key()),
                ));
            }
            Entry::Vacant(entry) => {
                // Emit `let quote_var_Name: <SwcType> = <value>;`.
                let var_ident = syn::Ident::new(&format!("quote_var_{name}"), Span::mixed_site());
                bindings.push(parse_quote! {
                    let #var_ident = #value;
                });
                entry.insert(VarData {
                    ident: var_ident,
                    ty: ty.clone(),
                });
            }
        }
    }

    let docs = {
        let (leading, trailing) = comments.borrow_all();
        jsdoc_comments(&leading, &trailing)
            .map(|comment| (comment.span.lo, comment.clone()))
            .collect()
    };

    let context = Context {
        span_expr: input.span.clone(),
        comments_expr: input.comments.clone(),
        vars,
        stand_ins,
        docs: RefCell::new(docs),
    };

    Ok((bindings, context))
}

/// Returns an iterator over all JSDoc comments in
/// a [`SingleThreadedComments`].
///
/// [`swc_ecma_parser`] treats a same-line block comment as
/// a trailing comment on the previous line, rather than
/// a leading comment on the current line, so we need to
/// look at all comments.
#[inline]
pub(crate) fn jsdoc_comments<'a>(
    leading: &'a SingleThreadedCommentsMapInner,
    trailing: &'a SingleThreadedCommentsMapInner,
) -> impl Iterator<Item = &'a Comment> {
    leading
        .values()
        .chain(trailing.values())
        .flatten()
        .filter(|comment| {
            // JSDoc comments start with `/**`, so the comment text
            // (everything after `/*`) should start with exactly one `*`.
            comment
                .text
                .strip_prefix('*')
                .is_some_and(|s| !s.starts_with('*'))
        })
}

/// Context for variable substitution during code generation.
pub(crate) struct Context {
    /// The optional `span` argument passed to the macro.
    span_expr: Option<syn::Expr>,
    /// The optional `comments` argument passed to the macro.
    comments_expr: Option<syn::Expr>,
    /// The variables passed to the macro.
    vars: HashMap<VarName, VarData>,
    /// Maps stand-ins to variable names.
    stand_ins: HashMap<String, StandInData>,
    /// JSDoc comments collected from the parse phase, keyed by
    /// the comment's start position.
    docs: RefCell<BTreeMap<BytePos, Comment>>,
}

impl Context {
    /// Looks up a variable by its stand-in (e.g., `__tsq_0__`).
    #[inline]
    pub fn stand_in(&self, value: &str) -> Option<&VarData> {
        let data = self.stand_ins.get(value)?;
        self.vars.get(&data.var)
    }

    /// Returns an expression for the [`Span`][swc_common::Span] to use
    /// in the generated Rust code.
    ///
    /// If a `span` expression was passed to the macro, returns that expression;
    /// otherwise, returns an expression that evaluates to
    /// [`DUMMY_SP`][swc_common::Span].
    #[inline]
    pub fn span(&self) -> syn::Expr {
        match &self.span_expr {
            Some(expr) => expr.clone(),
            None => syn::parse_quote!(::quasiquodo::ts::swc::common::DUMMY_SP),
        }
    }

    /// Returns the `comments` expression passed to the macro.
    #[inline]
    pub fn comments(&self) -> Option<&syn::Expr> {
        self.comments_expr.as_ref()
    }

    /// Consumes and returns the JSDoc comment closest to and preceding `lo`,
    /// if any.
    ///
    /// The heuristic of "the last JSDoc comment whose start position
    /// is before `lo`" handles both leading comments, which are
    /// preceded by a line break; and trailing comments, which are
    /// treated as part of the previous line even if they precede
    /// a token on the current line.
    #[inline]
    pub fn take_closest_doc_to(&self, lo: BytePos) -> Option<Comment> {
        let mut map = self.docs.borrow_mut();
        let (&key, _) = map.range(..lo).next_back()?;
        map.remove(&key)
    }
}

/// A variable name.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct VarName(String);

impl VarName {
    #[inline]
    pub fn from_str(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl Display for VarName {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Data for a single substitution variable.
pub(crate) struct VarData {
    /// The identifier for the `let` binding in the generated block.
    pub ident: syn::Ident,
    /// The declared type of this variable.
    pub ty: VarType,
}

/// Data for a single stand-in in the preprocessed source.
pub(crate) struct StandInData {
    /// The variable name, corresponding to [`Context::vars`].
    pub var: VarName,
}

#[derive(Debug, thiserror::Error)]
#[error("variable `#{{{0}}}` not bound to a value")]
pub struct UnboundVar(pub String);
