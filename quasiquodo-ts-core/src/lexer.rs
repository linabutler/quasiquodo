use std::collections::HashMap;
use std::ops::Range;

use swc_common::comments::{Comment, SingleThreadedComments, SingleThreadedCommentsMapInner};
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::{EsVersion, Ident};
use swc_ecma_parser::{Lexer, StringInput, Syntax, TsSyntax};
use winnow::Parser;

use super::{
    context::{PlaceholderData, VarName},
    input::{VarType, Variable},
};

/// Preprocesses a TypeScript source string, replacing `$var` markers
/// with type-appropriate placeholders prior to parsing.
///
/// For `LitStr` variables, `$name` becomes `"__tsq_N__"`, so that
/// [`swc_ecma_parser`] can parse it as a string literal. For all other types,
/// `$name` becomes an identifier like `__tsq_N__`.
///
/// Preprocessing also scans JSDoc-style comments (`/** ... */`) for
/// `$var` markers. Variables in JSDoc comments must be `LitStr` or
/// `Option<LitStr>`, and their markers become bare `__tsq_N__` placeholders.
///
/// A `$$` escapes a literal `$`: `$$` becomes `$` after preprocessing,
/// and `$$ident` becomes `$ident`.
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

    // Use SWC's lexer to scan the source for `$var` references and
    // `$$` escapes, and collect comments so that we can do the same.
    let source_map = Lrc::new(SourceMap::default());
    let source_file = source_map.new_source_file(FileName::Anon.into(), source.to_owned());
    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax::default()),
        EsVersion::latest(),
        StringInput::from(&*source_file),
        Some(&comments),
    );

    for t in lexer.filter(|t| t.token.is_word()) {
        let (start, end) = source_map.span_to_char_offset(&source_file, t.span);
        let start = usize::try_from(start).unwrap();
        let end = usize::try_from(end).unwrap();
        let Ok(token) = dollar::token.parse(&source[start..end]) else {
            continue;
        };

        match token {
            dollar::Token::Escape(suffix) => {
                // Escaped dollar sign: replace `$$suffix` with `$suffix`.
                replacements.push((format!("${suffix}"), start..end));
            }
            dollar::Token::Variable(name) => {
                let replacement = match variables.get(name) {
                    Some(&(index, ty)) => {
                        let placeholder = format!("__tsq_{index}__");
                        let replacement = match ty.inner() {
                            VarType::LitStr => format!(r#""{placeholder}""#),
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
                    None => String::new(),
                };
                replacements.push((replacement, start..end));
            }
        }
    }

    {
        // Scan all collected JSDoc-style comments for `$var` references
        // and `$$` escapes.
        let (leading, trailing) = comments.borrow_all();
        for comment in docs::comments(&leading, &trailing) {
            // The span includes the `/* ... */` delimiters in
            // the source file, so add 2 for offsets.
            let (start, _) = source_map.span_to_char_offset(&source_file, comment.span);
            let offset = usize::try_from(start).unwrap() + 2;

            // Insert placeholders for all `$var` references and
            // unescape all `$$` sequences.
            for (token, span) in dollar::tokens(&comment.text) {
                let replacement = match token {
                    dollar::Token::Escape(s) => format!("${s}"),
                    dollar::Token::Variable(name) => match variables.get(name) {
                        Some(&(index, ty)) => match ty.inner() {
                            // Only string splices (`LitStr` and `Option<LitStr>`)
                            // are allowed in JSDoc comments.
                            VarType::LitStr => {
                                let placeholder = format!("__tsq_{index}__");
                                placeholders.insert(
                                    placeholder.clone(),
                                    PlaceholderData {
                                        var: VarName::from_str(name),
                                    },
                                );
                                placeholder
                            }
                            _ => {
                                return Err(PreprocessError::JsDocVarType(name.to_owned(), ty));
                            }
                        },
                        None => String::new(),
                    },
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

pub(crate) mod docs {
    use super::*;

    use winnow::combinator::{alt, repeat};
    use winnow::token::{rest, take_until, take_while};

    /// A chunk of comment text.
    #[derive(Debug, PartialEq)]
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

pub(crate) mod dollar {
    use super::*;

    use winnow::{
        combinator::{alt, repeat},
        stream::{Compare, LocatingSlice, Stream, StreamIsPartial},
        token::{any, take_while},
    };

    /// A `$`-prefixed token recognized during preprocessing.
    #[derive(Debug)]
    pub enum Token<'a> {
        /// An escaped dollar sign, like `$$` or `$$ident`. The inner `&str` is
        /// the identifier continuation after `$$`, and can be empty.
        Escape(&'a str),
        /// A variable reference, like `$ident`. The inner `&str` is
        /// the identifier name, without the leading `$`.
        Variable(&'a str),
    }

    /// Collects all [`Token`]s in the source text, returning each token
    /// and its byte span.
    pub fn tokens(text: &str) -> Vec<(Token<'_>, Range<usize>)> {
        /// Consumes a `$` token at the current word boundary.
        #[inline]
        fn consume<'a>(
            input: &mut LocatingSlice<&'a str>,
        ) -> winnow::Result<Option<(Token<'a>, Range<usize>)>> {
            token.with_span().parse_next(input).map(Some)
        }

        /// Skips over non-`$` content, greedily consuming identifier runs
        /// until the next word boundary, to ensure that `$` only matches
        /// at word boundaries, not inside words.
        #[inline]
        fn skip<'a>(
            input: &mut LocatingSlice<&'a str>,
        ) -> winnow::Result<Option<(Token<'a>, Range<usize>)>> {
            alt((take_while(1.., Ident::is_valid_continue).void(), any.void()))
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
    pub fn token<'a, I>(input: &mut I) -> winnow::Result<Token<'a>>
    where
        I: Stream<Slice = &'a str, Token = char> + StreamIsPartial + Compare<I::Token>,
    {
        alt((
            ('$', '$', take_while(0.., Ident::is_valid_continue))
                .map(|(_, _, suffix)| Token::Escape(suffix)),
            ('$', take_while(1.., Ident::is_valid_continue)).map(|(_, name)| Token::Variable(name)),
        ))
        .parse_next(input)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PreprocessError<'a> {
    #[error("variable `${0}` in JSDoc comment must have type `LitStr`, but has type `{1:?}`")]
    JsDocVarType(String, &'a VarType),
}

#[cfg(test)]
mod tests {
    use super::{
        docs::{CommentSegment, segments},
        dollar::{Token, tokens},
    };

    // MARK: `$` word boundaries

    #[test]
    fn test_tokens_variable_at_word_boundary() {
        let result = tokens("foo $bar baz");
        assert_eq!(result.len(), 1);
        let (ref tok, ref span) = result[0];
        let Token::Variable(name) = tok else {
            panic!("expected `Token::Variable`; got `{tok:?}`")
        };
        assert_eq!(*name, "bar");
        assert_eq!(*span, 4..8);
    }

    #[test]
    fn test_tokens_variable_not_at_word_boundary() {
        let result = tokens("foo$barbaz");
        assert!(result.is_empty());
    }

    #[test]
    fn test_tokens_variable_at_start() {
        let result = tokens("$bar");
        assert_eq!(result.len(), 1);
        let (ref tok, ref span) = result[0];
        let Token::Variable(name) = tok else {
            panic!("expected `Token::Variable`; got `{tok:?}`")
        };
        assert_eq!(*name, "bar");
        assert_eq!(*span, 0..4);
    }

    #[test]
    fn test_tokens_escape_at_word_boundary() {
        let result = tokens("foo $$bar");
        assert_eq!(result.len(), 1);
        let (ref tok, ref span) = result[0];
        let Token::Escape(suffix) = tok else {
            panic!("expected `Token::Escape`; got `{tok:?}`")
        };
        assert_eq!(*suffix, "bar");
        assert_eq!(*span, 4..9);
    }

    #[test]
    fn test_tokens_escape_not_at_word_boundary() {
        let result = tokens("foo$$bar");
        assert!(result.is_empty());
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
