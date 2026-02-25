use std::collections::HashMap;

use swc_common::{FileName, SourceMap, comments::SingleThreadedComments, sync::Lrc};
use swc_ecma_ast::EsVersion;
use swc_ecma_parser::{Lexer, StringInput, Syntax, TsSyntax};

use super::{
    context::{StandInData, UnboundVar, VarName, jsdoc_comments},
    input::{VarType, Variable},
};

/// Preprocesses a TypeScript source string, replacing `#{var}`
/// placeholders with type-appropriate stand-ins prior to parsing:
///
/// * For `&str` and `String` variables, `#{var}` becomes `"__tsq_N__"`,
///   because positions like `ImportSpecifier` require string literals.
/// * For `Decl` variables, `#{var}` becomes `var __tsq_N__`,
///   because positions like `ExportDecl` require declarations.
/// * For `JsDoc` variables in source positions,
///   `#{var}` becomes `/** __tsq_N__ */`.
/// * For all other variables, and inside JSDoc comments,
///   `#{var}` becomes an identifier like `__tsq_N__`.
///
/// Returns the preprocessed source and a map of stand-ins to their variables.
pub(crate) fn preprocess(
    mut source: String,
    variables: &[Variable],
) -> Result<(String, HashMap<String, StandInData>), PreprocessError<'_>> {
    use placeholders::{CommentScanner, SourceScanner};

    let variables: HashMap<_, _> = variables
        .iter()
        .enumerate()
        .map(|(i, v)| (v.name.to_string(), (i, &v.ty)))
        .collect();

    let mut stand_ins = HashMap::new();
    let mut replacements = vec![];

    // Use SWC's lexer to scan the source for `#{var}` placeholders,
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

    // Scan the source for `#{var}` placeholders.
    for token in SourceScanner::new(&source_map, &all_tokens)
        .into_iter()
        .flatten()
    {
        let replacement = match variables.get(&*token.name) {
            Some(&(index, ty)) => {
                let stand_in = format!("__tsq_{index}__");
                let replacement = match ty.inner() {
                    VarType::Str(_) => format!(r#""{stand_in}""#),
                    VarType::JsDoc => format!("/** {stand_in} */"),
                    VarType::Decl => format!("var {stand_in}"),
                    // Identifiers are valid in all other positions.
                    _ => stand_in.clone(),
                };
                stand_ins.insert(
                    stand_in,
                    StandInData {
                        var: VarName::from_str(&token.name),
                    },
                );
                replacement
            }
            None => return Err(UnboundVar(token.name.into_owned()))?,
        };
        replacements.push((replacement, token.span));
    }

    {
        let (leading, trailing) = comments.borrow_all();
        for comment in jsdoc_comments(&leading, &trailing) {
            // The offset of each placeholder in the comment text is the start
            // position of the comment, plus 2 for the opening `/*` delimiter.
            let offset = (comment.span.lo - source_file.start_pos).0 as usize + 2;

            // Scan each JSDoc comment for `#{var}` placeholders.
            for token in CommentScanner::new(&comment.text) {
                let replacement = match variables.get(&*token.name) {
                    Some(&(index, ty)) => {
                        // Only string (`&str`, `String`) and `JsDoc`
                        // variables can be spliced into JSDoc comments.
                        if matches!(ty, VarType::Str(_) | VarType::JsDoc)
                            || matches!(ty, VarType::Option(inner)
                                if matches!(**inner, VarType::Str(_) | VarType::JsDoc))
                        {
                            let stand_in = format!("__tsq_{index}__");
                            stand_ins.insert(
                                stand_in.clone(),
                                StandInData {
                                    var: VarName::from_str(&token.name),
                                },
                            );
                            stand_in
                        } else {
                            return Err(PreprocessError::JsDocVarType(token.name.into_owned(), ty));
                        }
                    }
                    None => {
                        return Err(PreprocessError::JsDocUnboundVar(UnboundVar(
                            token.name.into_owned(),
                        )));
                    }
                };
                replacements.push((
                    replacement,
                    offset + token.span.start..offset + token.span.end,
                ));
            }
        }
    }

    // Sort the (guaranteed non-overlapping) spans found by both scanners
    // in source order, then replace back-to-front, so that earlier replacements
    // don't invalidate later spans.
    replacements.sort_by_key(|(_, span)| span.start);
    for (replacement, span) in replacements.into_iter().rev() {
        source.replace_range(span, &replacement);
    }

    Ok((source, stand_ins))
}

pub(crate) mod placeholders {
    use std::{borrow::Cow, ops::Range};

    use swc_common::{BytePos, SourceFile, SourceMap, sync::Lrc};
    use swc_ecma_ast::Ident;
    use swc_ecma_parser::unstable::{Token, TokenAndSpan};
    use winnow::{
        LocatingSlice, Parser,
        combinator::alt,
        token::{any, one_of, take_while},
    };

    /// A `#{var}` placeholder found by the [`SourceScanner`].
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct PlaceholderToken<'a> {
        /// The variable name, without the surrounding `#{}`.
        pub name: Cow<'a, str>,
        /// The byte span of the entire placeholder, including
        /// the surrounding `#{}`.
        pub span: Range<usize>,
    }

    impl<'a> PlaceholderToken<'a> {
        #[inline]
        pub fn new(name: impl Into<Cow<'a, str>>, span: Range<usize>) -> Self {
            let name = name.into();
            Self { name, span }
        }
    }

    /// An iterator that scans a TypeScript token stream for `#{var}`
    /// placeholders, yielding a [`PlaceholderToken`] for each.
    pub struct SourceScanner<'a> {
        source_file: Lrc<SourceFile>,
        tokens: &'a [TokenAndSpan],
    }

    impl<'a> SourceScanner<'a> {
        #[inline]
        pub fn new(source_map: &SourceMap, tokens: &'a [TokenAndSpan]) -> Option<Self> {
            if let [first, ..] = tokens
                && let Ok(Some(source_file)) = source_map.try_lookup_source_file(first.span.lo)
            {
                Some(Self {
                    source_file,
                    tokens,
                })
            } else {
                // No tokens; nothing to scan.
                None
            }
        }

        /// Converts a span position to a byte offset in the source file.
        #[inline]
        fn offset(&self, pos: BytePos) -> usize {
            (pos - self.source_file.start_pos).0 as usize
        }
    }

    impl<'a> Iterator for SourceScanner<'a> {
        type Item = PlaceholderToken<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                match self.tokens {
                    [hash, lbrace, name, rbrace, rest @ ..]
                        if matches!(hash.token, Token::Hash)
                            && matches!(lbrace.token, Token::LBrace)
                            && name.token.is_word()
                            && matches!(rbrace.token, Token::RBrace)
                            && hash.span.hi == lbrace.span.lo
                            && lbrace.span.hi == name.span.lo
                            && name.span.hi == rbrace.span.lo =>
                    {
                        self.tokens = rest;
                        let source = self.source_file.src.as_str();
                        let name = &source[self.offset(name.span.lo)..self.offset(name.span.hi)];
                        let start = self.offset(hash.span.lo);
                        let end = self.offset(rbrace.span.hi);
                        return Some(PlaceholderToken::new(name.to_owned(), start..end));
                    }
                    [_, rest @ ..] => self.tokens = rest,
                    [] => return None,
                }
            }
        }
    }

    /// An iterator that scans comment text for `#{var}` placeholders,
    /// yielding a [`PlaceholderToken`] for each.
    pub struct CommentScanner<'a> {
        input: LocatingSlice<&'a str>,
    }

    impl<'a> CommentScanner<'a> {
        #[inline]
        pub fn new(text: &'a str) -> Self {
            Self {
                input: LocatingSlice::new(text),
            }
        }
    }

    impl<'a> Iterator for CommentScanner<'a> {
        type Item = PlaceholderToken<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            #[inline]
            fn token<'a>(input: &mut LocatingSlice<&'a str>) -> winnow::Result<&'a str> {
                (
                    '#',
                    '{',
                    (
                        one_of(Ident::is_valid_start),
                        take_while(0.., Ident::is_valid_continue),
                    )
                        .take(),
                    '}',
                )
                    .map(|(_, _, name, _)| name)
                    .parse_next(input)
            }
            loop {
                if self.input.is_empty() {
                    return None;
                }
                if let Ok((name, span)) = token.with_span().parse_next(&mut self.input) {
                    return Some(PlaceholderToken::new(name, span));
                }
                alt((take_while(1.., |c: char| c != '#').void(), any.void()))
                    .parse_next(&mut self.input)
                    .map_err(|_: ()| ()) // Infallible.
                    .unwrap();
            }
        }
    }
}

pub(crate) mod stand_ins {
    use swc_ecma_ast::Ident;
    use winnow::{
        Parser,
        combinator::alt,
        token::{rest, take_until, take_while},
    };

    /// A token found by the [`StandInScanner`].
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum StandInToken<'a> {
        /// A stand-in like `__tsq_0__`.
        StandIn(&'a str),
        /// Literal text between stand-ins.
        Text(&'a str),
    }

    /// An iterator that scans text for `__tsq_N__` stand-ins,
    /// yielding a [`StandInToken`] for each.
    pub struct StandInScanner<'a> {
        input: &'a str,
    }

    impl<'a> StandInScanner<'a> {
        #[inline]
        pub fn new(input: &'a str) -> Self {
            Self { input }
        }
    }

    impl<'a> Iterator for StandInScanner<'a> {
        type Item = StandInToken<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.input.is_empty() {
                return None;
            }
            Some(
                alt((
                    ("__tsq_", take_while(1.., '0'..='9'), "__")
                        .take()
                        .map(StandInToken::StandIn),
                    alt((
                        // Greedily consume TypeScript identifier runs, so that
                        // a `__tsq_N__` substring inside a larger word becomes
                        // a text segment, not a stand-in.
                        //
                        // `Ident::is_valid_continue` is a superset of
                        // `Ident::is_valid_start`, so we only need to check it.
                        take_while(1.., Ident::is_valid_continue),
                        take_until(1.., "__tsq_"),
                        rest,
                    ))
                    .map(StandInToken::Text),
                ))
                .parse_next(&mut self.input)
                .map_err(|_: ()| ()) // Infallible.
                .unwrap(),
            )
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PreprocessError<'a> {
    #[error(transparent)]
    UnboundVar(#[from] UnboundVar),
    #[error(
        "variable `#{{{0}}}` in JSDoc comment must have type \
         `&str`, `String`, or `JsDoc`, but has type `{1}`"
    )]
    JsDocVarType(String, &'a VarType),
    #[error("variable `#{{{0}}}` in JSDoc comment not bound to a value")]
    JsDocUnboundVar(UnboundVar),
}

#[cfg(test)]
mod tests {
    use swc_common::{FileName, SourceMap, sync::Lrc};
    use swc_ecma_ast::EsVersion;
    use swc_ecma_parser::{Lexer, StringInput, Syntax, TsSyntax};

    use super::{
        placeholders::{CommentScanner, PlaceholderToken, SourceScanner},
        stand_ins::{StandInScanner, StandInToken},
    };

    // MARK: Source

    /// Lexes TypeScript source with SWC, and collects all `#{var}`
    /// placeholder tokens.
    fn scan_source(source: &str) -> Vec<PlaceholderToken<'static>> {
        let source_map = Lrc::new(SourceMap::default());
        let source_file = source_map.new_source_file(FileName::Anon.into(), source.to_owned());
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax::default()),
            EsVersion::latest(),
            StringInput::from(&*source_file),
            None,
        );
        let tokens: Vec<_> = lexer.collect();
        SourceScanner::new(&source_map, &tokens)
            .into_iter()
            .flatten()
            .map(|token| PlaceholderToken::new(token.name.into_owned(), token.span))
            .collect()
    }

    #[test]
    fn test_source_scanner_single_placeholder() {
        let result = scan_source("foo(#{bar})");
        assert_eq!(result, vec![PlaceholderToken::new("bar", 4..10)]);
    }

    #[test]
    fn test_source_scanner_multiple_placeholders() {
        let result = scan_source("#{a} + #{b}");
        assert_eq!(
            result,
            vec![
                PlaceholderToken::new("a", 0..4),
                PlaceholderToken::new("b", 7..11),
            ]
        );
    }

    #[test]
    fn test_source_scanner_no_placeholders() {
        let source_map = Lrc::new(SourceMap::default());
        assert!(SourceScanner::new(&source_map, &[]).is_none());

        let result = scan_source("foo + bar");
        assert!(result.is_empty());

        // A space between `#` and `{` means the spans aren't adjacent.
        let result = scan_source("# {bar}");
        assert!(result.is_empty());
    }

    #[test]
    fn test_source_scanner_placeholder_at_start() {
        let result = scan_source("#{x}");
        assert_eq!(result, vec![PlaceholderToken::new("x", 0..4)]);
    }

    #[test]
    fn test_source_scanner_placeholder_at_end() {
        let result = scan_source("foo + #{x}");
        assert_eq!(result, vec![PlaceholderToken::new("x", 6..10)]);
    }

    // MARK: Comments

    #[test]
    fn test_comment_scanner_single_placeholder() {
        let result: Vec<_> = CommentScanner::new("#{foo}").collect();
        assert_eq!(result, vec![PlaceholderToken::new("foo", 0..6)]);
    }

    #[test]
    fn test_comment_scanner_multiple_placeholders() {
        let result: Vec<_> = CommentScanner::new("#{a} and #{b}").collect();
        assert_eq!(
            result,
            vec![
                PlaceholderToken::new("a", 0..4),
                PlaceholderToken::new("b", 9..13),
            ]
        );
    }

    #[test]
    fn test_comment_scanner_no_placeholders() {
        let result: Vec<_> = CommentScanner::new("").collect();
        assert!(result.is_empty());

        let result: Vec<_> = CommentScanner::new("just text").collect();
        assert!(result.is_empty());

        // A `#` not followed by `{` should be skipped.
        let result: Vec<_> = CommentScanner::new("#foo #{bar}").collect();
        assert_eq!(result, vec![PlaceholderToken::new("bar", 5..11)]);

        // Digits can't start variable names.
        let result: Vec<_> = CommentScanner::new("#{0bad}").collect();
        assert!(result.is_empty());
    }

    #[test]
    fn test_comment_scanner_adjacent_placeholders() {
        let result: Vec<_> = CommentScanner::new("foo#{a}#{b}bar").collect();
        assert_eq!(
            result,
            vec![
                PlaceholderToken::new("a", 3..7),
                PlaceholderToken::new("b", 7..11),
            ]
        );
    }

    // MARK: Stand-ins

    #[test]
    fn test_stand_in_scanner_single_token() {
        let result: Vec<_> = StandInScanner::new("__tsq_0__").collect();
        assert_eq!(result, vec![StandInToken::StandIn("__tsq_0__")]);
    }

    #[test]
    fn test_stand_in_scanner_multiple_tokens() {
        let result: Vec<_> = StandInScanner::new("__tsq_0__ + __tsq_1__").collect();
        assert_eq!(
            result,
            vec![
                StandInToken::StandIn("__tsq_0__"),
                StandInToken::Text(" + "),
                StandInToken::StandIn("__tsq_1__"),
            ]
        );
    }

    #[test]
    fn test_stand_in_scanner_no_stand_ins() {
        let result: Vec<_> = StandInScanner::new("").collect();
        assert!(result.is_empty());

        // `__tsq___` has no digits, so it's not a stand-in.
        let result: Vec<_> = StandInScanner::new("__tsq___").collect();
        assert_eq!(result, vec![StandInToken::Text("__tsq___")]);

        // `__tsq_0` lacks the closing `__`.
        let result: Vec<_> = StandInScanner::new("__tsq_0").collect();
        assert_eq!(result, vec![StandInToken::Text("__tsq_0")]);
    }

    #[test]
    fn test_stand_in_scanner_at_word_boundary() {
        let result: Vec<_> = StandInScanner::new("* __tsq_0__ bar").collect();
        assert_eq!(
            result,
            vec![
                StandInToken::Text("* "),
                StandInToken::StandIn("__tsq_0__"),
                StandInToken::Text(" bar"),
            ]
        );
    }

    #[test]
    fn test_stand_in_scanner_not_at_word_boundary() {
        let result: Vec<_> = StandInScanner::new("foo__tsq_0__bar").collect();
        assert_eq!(result, vec![StandInToken::Text("foo__tsq_0__bar")]);
    }

    #[test]
    fn test_stand_in_scanner_adjacent_stand_ins() {
        let result: Vec<_> = StandInScanner::new("__tsq_0____tsq_1__").collect();
        // The first stand-in consumes `__tsq_0__`, then the
        // remaining `__tsq_1__` is a separate stand-in.
        assert_eq!(
            result,
            vec![
                StandInToken::StandIn("__tsq_0__"),
                StandInToken::StandIn("__tsq_1__"),
            ]
        );
    }
}
