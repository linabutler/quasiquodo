use std::collections::HashMap;
use std::ops::Range;

use swc_common::comments::{Comment, SingleThreadedComments, SingleThreadedCommentsMapInner};
use swc_common::sync::Lrc;
use swc_common::{BytePos, FileName, SourceMap};
use swc_ecma_ast::{EsVersion, Ident};
use swc_ecma_parser::unstable::{Token, TokenAndSpan};
use swc_ecma_parser::{Lexer, StringInput, Syntax, TsSyntax};
use winnow::Parser;

use super::{
    context::{PlaceholderData, UnboundVar, VarName},
    input::{VarType, Variable},
};

/// Preprocesses a TypeScript source string, replacing `@{var}`
/// markers with type-appropriate placeholders prior to parsing:
///
/// * For `LitStr` variables, `@{name}` becomes `"__tsq_N__"`, because
///   positions like `ImportSpecifier` require string literals.
/// * For `Decl` variables, `@{name}` becomes `var __tsq_N__`, because
///   positions like `ExportDecl` require declarations.
/// * For all other types, `@{name}` becomes an identifier like
///   `__tsq_N__`.
///
/// Preprocessing also scans JSDoc-style comments (`/** ... */`) for
/// `@{var}` markers. Variables in JSDoc comments must be `LitStr`, `JsDoc`,
/// or `Option<LitStr | JsDoc>`, and their markers become bare `__tsq_N__`
/// placeholders.
///
/// Returns the preprocessed source and a map from placeholder values
/// to variable data.
pub(crate) fn preprocess(
    mut source: String,
    variables: &[Variable],
) -> Result<(String, HashMap<String, PlaceholderData>), PreprocessError<'_>> {
    let variables: HashMap<_, _> = variables
        .iter()
        .enumerate()
        .map(|(i, v)| (v.name.to_string(), (i, &v.ty)))
        .collect();

    let mut placeholders = HashMap::new();
    let mut replacements = vec![];

    // Use SWC's lexer to scan the source for `@{var}` markers,
    // and collect comments so that we can scan them after.
    let source_map = Lrc::new(SourceMap::default());
    let source_file = source_map.new_source_file(FileName::Anon.into(), source.to_owned());
    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax::default()),
        EsVersion::latest(),
        StringInput::from(&*source_file),
        Some(&comments),
    );

    let all_tokens: Vec<_> = lexer.collect();
    let base = source_file.start_pos;

    for (name, span) in Markers::new(&all_tokens, &source, base) {
        let replacement = match variables.get(name) {
            Some(&(index, ty)) => {
                let placeholder = format!("__tsq_{index}__");
                let replacement = match ty.inner() {
                    VarType::LitStr => format!(r#""{placeholder}""#),
                    VarType::JsDoc => format!("/** {placeholder} */"),
                    // Bare identifiers aren't valid in all `Decl`
                    // positions (e.g., after `export`), so we use
                    // `var __tsq_N__` as the stand-in for these.
                    VarType::Decl => format!("var {placeholder}"),
                    // Identifiers are valid in all other positions.
                    _ => placeholder.clone(),
                };
                placeholders.insert(
                    placeholder,
                    PlaceholderData {
                        var: VarName::from_str(name),
                    },
                );
                replacement
            }
            None => return Err(UnboundVar(name.to_owned()))?,
        };
        replacements.push((replacement, span));
    }

    {
        // Scan all collected JSDoc-style comments for `@{var}`
        // references.
        let (leading, trailing) = comments.borrow_all();
        for comment in docs::comments(&leading, &trailing) {
            // The span includes the `/* ... */` delimiters in
            // the source file, so add 2 for offsets.
            let offset = (comment.span.lo - base).0 as usize + 2;

            // Insert placeholders for all `@{var}` references.
            for (name, span) in marker::tokens(&comment.text) {
                let replacement = match variables.get(name) {
                    Some(&(index, ty)) => {
                        // Only string (`LitStr`) and `JsDoc` splices
                        // are allowed in JSDoc comments.
                        if matches!(ty, VarType::LitStr | VarType::JsDoc)
                            || matches!(ty, VarType::Option(inner)
                                if matches!(**inner, VarType::LitStr | VarType::JsDoc))
                        {
                            let placeholder = format!("__tsq_{index}__");
                            placeholders.insert(
                                placeholder.clone(),
                                PlaceholderData {
                                    var: VarName::from_str(name),
                                },
                            );
                            placeholder
                        } else {
                            return Err(PreprocessError::JsDocVarType(name.to_owned(), ty));
                        }
                    }
                    None => {
                        return Err(PreprocessError::JsDocUnboundVar(UnboundVar(
                            name.to_owned(),
                        )));
                    }
                };
                replacements.push((replacement, offset + span.start..offset + span.end));
            }
        }
    }

    // Sort and deduplicate, then apply replacements
    // in reverse byte order to preserve offsets.
    replacements.sort_by_key(|(_, span)| span.start);
    replacements.dedup_by_key(|(_, span)| span.start);
    for (replacement, span) in replacements.into_iter().rev() {
        source.replace_range(span, &replacement);
    }

    Ok((source, placeholders))
}

/// An iterator over `@{name}` markers in a TypeScript token stream.
///
/// Recognizes adjacent `[@ { Word }]` token sequences, and yields
/// the variable name and byte range of each.
struct Markers<'a> {
    tokens: &'a [TokenAndSpan],
    source: &'a str,
    base: BytePos,
}

impl<'a> Markers<'a> {
    fn new(tokens: &'a [TokenAndSpan], source: &'a str, base: BytePos) -> Self {
        Self {
            tokens,
            source,
            base,
        }
    }

    /// Converts a span position to a byte offset in `source`.
    fn offset(&self, pos: BytePos) -> usize {
        (pos - self.base).0 as usize
    }
}

impl<'a> Iterator for Markers<'a> {
    type Item = (&'a str, Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.tokens {
                [at, lbrace, name, rbrace, rest @ ..]
                    if matches!(at.token, Token::At)
                        && matches!(lbrace.token, Token::LBrace)
                        && name.token.is_word()
                        && matches!(rbrace.token, Token::RBrace)
                        && at.span.hi == lbrace.span.lo
                        && lbrace.span.hi == name.span.lo
                        && name.span.hi == rbrace.span.lo =>
                {
                    self.tokens = rest;
                    let var = &self.source[self.offset(name.span.lo)..self.offset(name.span.hi)];
                    let start = self.offset(at.span.lo);
                    let end = self.offset(rbrace.span.hi);
                    return Some((var, start..end));
                }
                [_, rest @ ..] => self.tokens = rest,
                [] => return None,
            }
        }
    }
}

pub(crate) mod docs {
    use super::*;

    use winnow::combinator::{alt, repeat};
    use winnow::token::{rest, take_until, take_while};

    /// A chunk of comment text.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum CommentSegment<'a> {
        /// Literal text between placeholders.
        Text(&'a str),
        /// A placeholder, like `__tsq_0__`.
        Placeholder(&'a str),
    }

    /// Returns an iterator over all JSDoc-style comments in
    /// a [`SingleThreadedComments`].
    ///
    /// [`swc_ecma_parser`] treats a same-line block comment as
    /// a trailing comment on the previous line, rather than
    /// a leading comment on the current line, so we need to
    /// look at both.
    #[inline]
    pub fn comments<'a>(
        leading: &'a SingleThreadedCommentsMapInner,
        trailing: &'a SingleThreadedCommentsMapInner,
    ) -> impl Iterator<Item = &'a Comment> {
        leading
            .values()
            .chain(trailing.values())
            .flatten()
            .filter(|comment| {
                // JSDoc comments start with `/**`, so the comment text should
                // start with exactly one `*`.
                if let Some(suffix) = comment.text.strip_prefix('*')
                    && !suffix.starts_with('*')
                {
                    true
                } else {
                    false
                }
            })
    }

    /// Parses comment text into a sequence of [`CommentSegment`]s,
    /// splitting on `__tsq_N__` placeholders.
    pub fn segments<'a>(input: &'a str) -> Vec<CommentSegment<'a>> {
        repeat(
            0..,
            alt((
                placeholder.map(CommentSegment::Placeholder),
                alt((
                    // Greedily consume identifier runs, so that
                    // a `__tsq_N__` substring inside a larger word
                    // becomes a text segment, not a placeholder segment.
                    take_while(1.., Ident::is_valid_continue),
                    take_until(1.., "__tsq_"),
                    rest.verify(|s: &str| !s.is_empty()),
                ))
                .map(CommentSegment::Text),
            )),
        )
        .parse(input)
        .unwrap()
    }

    /// Recognizes and returns a `__tsq_N__` placeholder.
    #[inline]
    fn placeholder<'a>(input: &mut &'a str) -> winnow::Result<&'a str> {
        ("__tsq_", take_while(1.., '0'..='9'), "__")
            .take()
            .parse_next(input)
    }
}

pub(crate) mod marker {
    use super::*;

    use winnow::{
        combinator::{alt, repeat},
        stream::{Compare, LocatingSlice, Stream, StreamIsPartial},
        token::{any, take_while},
    };

    /// Collects all markers in the source text, returning each marker's
    /// identifier name (without `@{` and `}`) and byte span.
    pub fn tokens(text: &str) -> Vec<(&str, Range<usize>)> {
        /// Consumes a `@{name}` token.
        #[inline]
        fn consume<'a>(
            input: &mut LocatingSlice<&'a str>,
        ) -> winnow::Result<Option<(&'a str, Range<usize>)>> {
            token.with_span().parse_next(input).map(Some)
        }

        /// Skips over non-`@{...}` content.
        #[inline]
        fn skip<'a>(
            input: &mut LocatingSlice<&'a str>,
        ) -> winnow::Result<Option<(&'a str, Range<usize>)>> {
            alt((take_while(1.., |c: char| c != '@').void(), any.void()))
                .parse_next(input)
                .map(|()| None)
        }

        repeat(0.., alt((consume, skip)))
            .fold(Vec::new, |mut tokens, item| {
                tokens.extend(item);
                tokens
            })
            .parse(LocatingSlice::new(text))
            .unwrap()
    }

    /// Parses a single [`Token`] from the input.
    #[inline]
    pub fn token<'a, I>(input: &mut I) -> winnow::Result<I::Slice>
    where
        I: Stream<Slice = &'a str, Token = char> + StreamIsPartial + Compare<I::Token>,
    {
        ('@', '{', take_while(1.., Ident::is_valid_continue), '}')
            .map(|(_, _, name, _)| name)
            .parse_next(input)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PreprocessError<'a> {
    #[error(transparent)]
    UnboundVar(#[from] UnboundVar),
    #[error(
        "variable `@{{{0}}}` in JSDoc comment must have type \
         `LitStr` or `JsDoc`, but has type `{1:?}`"
    )]
    JsDocVarType(String, &'a VarType),
    #[error("variable `@{{{0}}}` in JSDoc comment not bound to a value")]
    JsDocUnboundVar(UnboundVar),
}

#[cfg(test)]
mod tests {
    use super::{
        docs::{CommentSegment, segments},
        marker::tokens,
    };

    // MARK: `@{...}` markers

    #[test]
    fn test_tokens_variable_surrounded_by_spaces() {
        let result = tokens("foo @{bar} baz");
        assert_eq!(result.len(), 1);
        let (name, ref span) = result[0];
        assert_eq!(name, "bar");
        assert_eq!(*span, 4..10);
    }

    #[test]
    fn test_tokens_variable_adjacent_to_text() {
        let result = tokens("foo@{bar}baz");
        assert_eq!(result.len(), 1);
        let (name, ref span) = result[0];
        assert_eq!(name, "bar");
        assert_eq!(*span, 3..9);
    }

    #[test]
    fn test_tokens_variable_at_start() {
        let result = tokens("@{bar}");
        assert_eq!(result.len(), 1);
        let (name, ref span) = result[0];
        assert_eq!(name, "bar");
        assert_eq!(*span, 0..6);
    }

    // MARK: Placeholder word boundaries

    #[test]
    fn test_comment_sources_placeholder_at_word_boundary() {
        let result = segments("* __tsq_0__ bar");
        assert_eq!(
            result,
            vec![
                CommentSegment::Text("* "),
                CommentSegment::Placeholder("__tsq_0__"),
                CommentSegment::Text(" bar"),
            ]
        );
    }

    #[test]
    fn test_comment_sources_placeholder_not_at_word_boundary() {
        let result = segments("foo__tsq_0__bar");
        assert_eq!(result, vec![CommentSegment::Text("foo__tsq_0__bar")]);
    }
}
