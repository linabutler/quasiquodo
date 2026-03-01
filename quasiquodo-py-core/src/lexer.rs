use std::collections::HashMap;

use super::{
    context::{StandInData, UnboundVar, VarName},
    input::{VarType, Variable},
};

/// Preprocesses a Python source string, replacing `#{var}` placeholders
/// with stand-ins prior to parsing.
///
/// Preprocessing uses a string-aware scanner to distinguish
/// placeholders in code position from placeholders inside string
/// literals:
///
/// * Code position placeholders become identifier stand-ins
///   (e.g., `__pyq_0__`).
/// * String position placeholders become `__pyq_N__` stand-in
///   substrings embedded in the string. Only `&str`, `String`,
///   and `Option<&str | String>` are allowed here.
///
/// Returns the preprocessed source and a map of stand-ins to their
/// variables.
pub(crate) fn preprocess(
    mut source: String,
    variables: &[Variable],
) -> Result<(String, HashMap<String, StandInData>), PreprocessError<'_>> {
    use scan::{Scanner, TokenPosition};

    let variables: HashMap<_, _> = variables
        .iter()
        .enumerate()
        .map(|(i, v)| (v.name.to_string(), (i, &v.ty)))
        .collect();

    let mut stand_ins = HashMap::new();
    let mut replacements = vec![];

    for m in Scanner::new(&source) {
        let replacement = match variables.get(m.name) {
            Some(&(index, ty)) => {
                // Validate type for string-position placeholders.
                if m.pos == TokenPosition::String && !ty.is_str() {
                    return Err(PreprocessError::StringVarType(m.name.to_owned(), ty));
                }
                let stand_in = format!("__pyq_{index}__");
                stand_ins.insert(
                    stand_in.clone(),
                    StandInData {
                        var: VarName::from_str(m.name),
                    },
                );
                stand_in
            }
            None => return Err(UnboundVar(m.name.to_owned()))?,
        };
        replacements.push((replacement, m.span));
    }

    for (replacement, span) in replacements.into_iter().rev() {
        source.replace_range(span, &replacement);
    }

    Ok((source, stand_ins))
}

/// A string-aware scanner for Python source text.
///
/// [Ruff's lexer][ruff_python_parser::lexer::Lexer] doesn't expose
/// token values or position information, so we implement our own pair of
/// (relatively) lightweight lexers to find `#{name}` placeholders in code
/// and string positions, including inside f- and t-string expression holes.
pub(crate) mod scan {
    use std::ops::Range;

    use logos::{Lexer, Logos};

    /// A placeholder found during scanning.
    #[derive(Clone, Debug)]
    pub struct PlaceholderToken<'a> {
        /// The variable name, without the surrounding `#{}`.
        pub name: &'a str,
        /// The byte span of the entire `#{name}` placeholder.
        pub span: Range<usize>,
        /// Whether the placeholder occurred in code or string position.
        pub pos: TokenPosition,
    }

    /// The position in which a placeholder was found.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum TokenPosition {
        /// The placeholder is in code position.
        Code,
        /// The placeholder is inside a string literal.
        String,
    }

    // MARK: Tokens

    /// Tokens recognized in code, either top-level or inside an f- or t-string
    /// expression hole.
    ///
    /// This lexer skips characters that aren't interesting for recognizing
    /// placeholders: whitespace, digits, and other punctuators.
    #[derive(Logos, Debug)]
    #[logos(skip r#"[^#\p{XID_Continue}_'"{}]+"#)]
    #[logos(subpattern ident = r"[\p{XID_Start}_]\p{XID_Continue}*")]
    #[logos(subpattern interp2 = r"(?:[rR][fFtT]|[fFtT][rR])")]
    #[logos(subpattern plain2 = r"(?:[rR][bB]|[bB][rR])")]
    #[logos(subpattern interp1 = r"[fFtT]")]
    #[logos(subpattern plain1 = r"[rRbBuU]")]
    enum CodeToken<'a> {
        /// A placeholder like `#{name}`.
        #[regex(r#"#\{(?&ident)\}"#, |lex| &lex.slice()[2..lex.slice().len() - 1])]
        Placeholder(&'a str),

        /// A `#` line comment.
        #[token("#", skip_comment)]
        Comment,

        // Two-character prefix; interpolated.
        #[regex(r#"(?&interp2)'"#, |_| StringStart(Interp::Yes, Quote::Single, Triple::No))]
        #[regex(r#"(?&interp2)'{3}"#, |_| StringStart(Interp::Yes, Quote::Single, Triple::Yes))]
        #[regex(r#"(?&interp2)""#, |_| StringStart(Interp::Yes, Quote::Double, Triple::No))]
        #[regex(r#"(?&interp2)"{3}"#, |_| StringStart(Interp::Yes, Quote::Double, Triple::Yes))]
        // Two-character prefix; not interpolated.
        #[regex(r#"(?&plain2)'"#, |_| StringStart(Interp::No, Quote::Single, Triple::No))]
        #[regex(r#"(?&plain2)'{3}"#, |_| StringStart(Interp::No, Quote::Single, Triple::Yes))]
        #[regex(r#"(?&plain2)""#, |_| StringStart(Interp::No, Quote::Double, Triple::No))]
        #[regex(r#"(?&plain2)"{3}"#, |_| StringStart(Interp::No, Quote::Double, Triple::Yes))]
        // One-character prefix; interpolated.
        #[regex(r#"(?&interp1)'"#, |_| StringStart(Interp::Yes, Quote::Single, Triple::No))]
        #[regex(r#"(?&interp1)'{3}"#, |_| StringStart(Interp::Yes, Quote::Single, Triple::Yes))]
        #[regex(r#"(?&interp1)""#, |_| StringStart(Interp::Yes, Quote::Double, Triple::No))]
        #[regex(r#"(?&interp1)"{3}"#, |_| StringStart(Interp::Yes, Quote::Double, Triple::Yes))]
        // One-character prefix; not interpolated.
        #[regex(r#"(?&plain1)'"#, |_| StringStart(Interp::No, Quote::Single, Triple::No))]
        #[regex(r#"(?&plain1)'{3}"#, |_| StringStart(Interp::No, Quote::Single, Triple::Yes))]
        #[regex(r#"(?&plain1)""#, |_| StringStart(Interp::No, Quote::Double, Triple::No))]
        #[regex(r#"(?&plain1)"{3}"#, |_| StringStart(Interp::No, Quote::Double, Triple::Yes))]
        // Unprefixed.
        #[regex("'", |_| StringStart(Interp::No, Quote::Single, Triple::No))]
        #[regex("'{3}", |_| StringStart(Interp::No, Quote::Single, Triple::Yes))]
        #[regex(r#"""#, |_| StringStart(Interp::No, Quote::Double, Triple::No))]
        #[regex(r#""{3}"#, |_| StringStart(Interp::No, Quote::Double, Triple::Yes))]
        /// The start of a string, with or without a prefix. [`StringStart`] is
        /// computed by the callback for each start pattern.
        String(StringStart),

        #[token("{")]
        BraceOpen,

        #[token("}")]
        BraceClose,
    }

    fn skip_comment<'a>(lex: &mut Lexer<'a, CodeToken<'a>>) {
        let remainder = lex.remainder();
        let n = if let Some(index) = remainder.find('\n') {
            // This branch catches LF and CRLF endings. Since we
            // skip comments, the distinction doesn't matter.
            index + 1
        } else if let Some(index) = remainder.find('\r') {
            index + 1
        } else {
            remainder.len()
        };
        lex.bump(n);
    }

    /// Tokens recognized inside a string literal body.
    ///
    /// This lexer doesn't validate escape sequences, and skips characters that
    /// aren't interesting for recognizing placeholders and expression holes.
    #[derive(Logos, Debug)]
    #[logos(skip r#"[^#\\'"{}]+"#)]
    #[logos(subpattern ident = r"[\p{XID_Start}_]\p{XID_Continue}*")]
    enum StringToken<'a> {
        /// A placeholder like `#{name}`. Placeholders are only recognized
        /// inside interpolated (f- and t-) strings, and triple-quoted strings.
        #[regex(r#"#\{(?&ident)\}"#, |lex| &lex.slice()[2..lex.slice().len() - 1])]
        Placeholder(&'a str),

        /// A backslash followed by any character.
        ///
        /// Escapes determine whether to continue or close the string.
        ///
        /// Our handling of escapes diverges from Python's in one case:
        /// Python lexes a named Unicode character (like `\N{SNOWMAN}`)
        /// in an f-string as an escape sequence, while we lex `\N` as
        /// an `Escape`, and `{SNOWMAN}` as an expression hole. This is
        /// technically wrong, but self-healing: Unicode character names
        /// only ever contain ASCII letters, digits, spaces, and hyphens,
        /// which the code lexer consumes; then `}` closes the phantom hole
        /// at the correct byte offset. We're only scanning for placeholders,
        /// so this difference isn't semantically meaningful.
        #[regex(r#"\\."#)]
        Escape,

        #[token("'", |_| StringEnd(Quote::Single, Triple::No))]
        #[regex("'{3}", |_| StringEnd(Quote::Single, Triple::Yes))]
        #[token(r#"""#, |_| StringEnd(Quote::Double, Triple::No))]
        #[regex(r#""{3}"#, |_| StringEnd(Quote::Double, Triple::Yes))]
        /// The end of the string literal.
        End(StringEnd),

        /// An escaped opening brace inside an f- or t-string.
        #[token("{{")]
        EscapedBraceOpen,

        /// An escaped closing brace inside an f- or t-string.
        #[token("}}")]
        EscapedBraceClose,

        /// An opening brace for an expression hole inside an f- or t-string.
        #[token("{")]
        BraceOpen,

        /// A closing brace for an expression hole inside an f- or t-string.
        #[token("}")]
        BraceClose,
    }

    /// Whether a string is triple-quoted (`'''` or `"""`) or not.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Triple {
        Yes,
        No,
    }

    /// Whether a string is interpolated (f-string or t-string).
    #[derive(Clone, Copy, Debug)]
    enum Interp {
        Yes,
        No,
    }

    /// A string's quote delimiter.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Quote {
        Single,
        Double,
    }

    /// What kind of string are we about to enter?
    #[derive(Clone, Copy, Debug)]
    struct StringStart(Interp, Quote, Triple);

    /// What kind of string did we just leave?
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct StringEnd(Quote, Triple);

    // MARK: Scanner

    /// Code lexer context, passed between [`Active::Code`]
    /// and [`Suspended::code`].
    #[derive(Clone, Copy, Debug, Default)]
    struct CodeContext {
        in_expr_hole: bool,
        brace_depth: usize,
    }

    /// The currently active lexer context.
    #[derive(Debug)]
    enum Active<'a> {
        /// Code; top-level or within an expression hole.
        Code(Lexer<'a, CodeToken<'a>>, CodeContext),
        /// String literal body.
        String(Lexer<'a, StringToken<'a>>, StringStart),
    }

    /// Suspended lexer contexts.
    #[derive(Debug, Default)]
    struct Suspended {
        /// Suspended code context state, pushed when entering a string.
        code: Vec<CodeContext>,
        /// Suspended string context state, pushed when entering an
        /// expression hole in an f- or t-string.
        string: Vec<StringStart>,
    }

    /// An iterator that scans Python source text for `#{name}` placeholders,
    /// yielding a [`PlaceholderToken`] for each. Tracks string boundaries and
    /// expression holes in alternating stacks.
    pub struct Scanner<'a> {
        active: Option<Active<'a>>,
        suspended: Suspended,
    }

    impl<'a> Scanner<'a> {
        /// Creates a new scanner for the given source text.
        pub fn new(source: &'a str) -> Self {
            Self {
                active: Some(Active::Code(
                    CodeToken::lexer(source),
                    CodeContext::default(),
                )),
                suspended: Suspended::default(),
            }
        }
    }

    impl<'a> Iterator for Scanner<'a> {
        type Item = PlaceholderToken<'a>;

        fn next(&mut self) -> Option<PlaceholderToken<'a>> {
            loop {
                match self.active.take()? {
                    Active::Code(mut lex, mut context) => {
                        let Ok(token) = lex.next()? else {
                            self.active = Some(Active::Code(lex, context));
                            continue;
                        };
                        match token {
                            CodeToken::Placeholder(name) => {
                                let span = lex.span();
                                self.active = Some(Active::Code(lex, context));
                                return Some(PlaceholderToken {
                                    name,
                                    span: span.start..span.end,
                                    pos: TokenPosition::Code,
                                });
                            }
                            CodeToken::String(start) => {
                                self.suspended.code.push(context);
                                self.active = Some(Active::String(lex.morph(), start));
                            }
                            CodeToken::BraceClose
                                if context.in_expr_hole && context.brace_depth == 0 =>
                            {
                                let start = self.suspended.string.pop()?;
                                self.active = Some(Active::String(lex.morph(), start));
                            }
                            token => {
                                if context.in_expr_hole {
                                    match token {
                                        CodeToken::BraceOpen => {
                                            context.brace_depth += 1;
                                        }
                                        CodeToken::BraceClose => {
                                            context.brace_depth -= 1;
                                        }
                                        _ => (),
                                    }
                                }
                                self.active = Some(Active::Code(lex, context));
                            }
                        }
                    }

                    Active::String(mut lex, start) => {
                        let Ok(token) = lex.next()? else {
                            self.active = Some(Active::String(lex, start));
                            continue;
                        };
                        match (start, token) {
                            // `#{...}` placeholder.
                            (
                                // Only treat as a placeholder if it's inside a
                                // triple-quoted or interpolated string.
                                StringStart(_, _, Triple::Yes)
                                | StringStart(Interp::Yes, _, Triple::No),
                                StringToken::Placeholder(name),
                            ) => {
                                let span = lex.span();
                                self.active = Some(Active::String(lex, start));
                                return Some(PlaceholderToken {
                                    name,
                                    span: span.start..span.end,
                                    pos: TokenPosition::String,
                                });
                            }
                            // Matching (delimiter and triples) closing quote.
                            (StringStart(_, a, c), StringToken::End(StringEnd(b, d)))
                                if a == b && c == d =>
                            {
                                let context = self.suspended.code.pop()?;
                                self.active = Some(Active::Code(lex.morph(), context));
                            }
                            // Opening `{` of an f- or t-string expression hole.
                            (StringStart(Interp::Yes, _, _), StringToken::BraceOpen) => {
                                self.suspended.string.push(start);
                                self.active = Some(Active::Code(
                                    lex.morph(),
                                    CodeContext {
                                        in_expr_hole: true,
                                        ..Default::default()
                                    },
                                ));
                            }
                            // Any other character is uninteresting to us.
                            _ => {
                                self.active = Some(Active::String(lex, start));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) mod stand_ins {
    use winnow::{
        Parser,
        combinator::alt,
        token::{rest, take_until, take_while},
    };

    /// A token found by the [`StandInScanner`].
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum StandInToken<'a> {
        /// A stand-in like `__pyq_0__`.
        StandIn(&'a str),
        /// Literal text between stand-ins.
        Text(&'a str),
    }

    /// An iterator that scans text for `__pyq_N__` stand-ins,
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
                    ("__pyq_", take_while(1.., '0'..='9'), "__")
                        .take()
                        .map(StandInToken::StandIn),
                    alt((
                        // Greedily consume Python identifier runs, so that
                        // a `__pyq_N__` substring inside a larger word becomes
                        // a text segment, not a stand-in.
                        //
                        // `is_xid_continue` is a superset of `is_xid_start`,
                        // so we only need to check it.
                        take_while(1.., unicode_ident::is_xid_continue),
                        take_until(1.., "__pyq_"),
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
        "variable `#{{{0}}}` in string literal must be \
         `&str` or `String`, but is `{1}`"
    )]
    StringVarType(String, &'a VarType),
}

#[cfg(test)]
mod tests {
    use super::{
        scan::{Scanner, TokenPosition},
        stand_ins::{StandInScanner, StandInToken},
    };

    // MARK: Stand-in word boundaries

    #[test]
    fn test_segments_stand_in_at_word_boundary() {
        let result: Vec<_> = StandInScanner::new("hello __pyq_0__ world").collect();
        assert_eq!(
            result,
            vec![
                StandInToken::Text("hello"),
                StandInToken::Text(" "),
                StandInToken::StandIn("__pyq_0__"),
                StandInToken::Text(" world"),
            ]
        );
    }

    #[test]
    fn test_segments_stand_in_not_at_word_boundary() {
        let result: Vec<_> = StandInScanner::new("foo__pyq_0__bar").collect();
        assert_eq!(result, vec![StandInToken::Text("foo__pyq_0__bar")]);
    }

    // MARK: Scanner, code

    #[test]
    fn test_scan_marker_in_code() {
        let result: Vec<_> = Scanner::new("x = #{v}").collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "v");
        assert_eq!(result[0].span, 4..8);
        assert_eq!(result[0].pos, TokenPosition::Code);
    }

    #[test]
    fn test_scan_multiple_markers_in_code() {
        let result: Vec<_> = Scanner::new("#{a} + #{b}").collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "a");
        assert_eq!(result[0].pos, TokenPosition::Code);
        assert_eq!(result[1].name, "b");
        assert_eq!(result[1].pos, TokenPosition::Code);
    }

    // MARK: Scanner, string

    #[test]
    fn test_scan_marker_in_double_quoted_string_ignored() {
        let result: Vec<_> = Scanner::new(r#"x = "hello #{v}""#).collect();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_scan_marker_in_single_quoted_string_ignored() {
        let result: Vec<_> = Scanner::new("x = 'hello #{v}'").collect();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_scan_marker_in_triple_quoted_string() {
        let result: Vec<_> = Scanner::new(r#"x = """hello #{v}""""#).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "v");
        assert_eq!(result[0].pos, TokenPosition::String);
    }

    #[test]
    fn test_scan_marker_in_raw_string_ignored() {
        let result: Vec<_> = Scanner::new(r#"r"hello #{v}""#).collect();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_scan_marker_in_byte_string_ignored() {
        let result: Vec<_> = Scanner::new(r#"b"hello #{v}""#).collect();
        assert_eq!(result.len(), 0);
    }

    // MARK: Scanner, mixed code and string

    #[test]
    fn test_scan_marker_in_plain_string_ignored() {
        // Only the code position marker should be found; the one
        // inside the plain double-quoted string should be ignored.
        let result: Vec<_> = Scanner::new(r#"#{a} = "hello #{b}""#).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "a");
        assert_eq!(result[0].pos, TokenPosition::Code);
    }

    // MARK: Scanner, comments

    #[test]
    fn test_scan_comments() {
        // Placeholder inside a comment is ignored.
        let result: Vec<_> = Scanner::new("x = 1  # #{v}").collect();
        assert_eq!(result.len(), 0);

        // Placeholder on the line after a comment is found.
        let result: Vec<_> = Scanner::new("# comment\n#{v}").collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "v");
        assert_eq!(result[0].pos, TokenPosition::Code);

        // Comment at EOF without trailing newline.
        let result: Vec<_> = Scanner::new("# just a comment").collect();
        assert_eq!(result.len(), 0);

        // Bare `#` at EOF.
        let result: Vec<_> = Scanner::new("#").collect();
        assert_eq!(result.len(), 0);

        // Comment terminated by `\r\n`.
        let result: Vec<_> = Scanner::new("# comment\r\n#{v}").collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "v");
        assert_eq!(result[0].pos, TokenPosition::Code);

        // Comment terminated by bare `\r`.
        let result: Vec<_> = Scanner::new("# comment\r#{v}").collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "v");
        assert_eq!(result[0].pos, TokenPosition::Code);
    }

    // MARK: Scanner, escapes

    #[test]
    fn test_scan_escaped_quote_in_string() {
        // The string `"hello \" #{v}"`. The escaped `\"` doesn't terminate
        // the string, so the placeholder inside should be ignored and
        // treated as text.
        let result: Vec<_> = Scanner::new(r#""hello \" #{v}""#).collect();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_scan_backslash_at_end_of_string() {
        // The string `"hello\\"`, followed by `#{v}` in code position.
        // `\\` is an escaped backslash; the following `"` closes the string.
        let result: Vec<_> = Scanner::new(r#""hello\\" #{v}"#).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pos, TokenPosition::Code);
    }

    // MARK: Scanner, f-strings

    #[test]
    fn test_scan_marker_in_fstring_text() {
        let result: Vec<_> = Scanner::new(r#"f"hello #{v}""#).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pos, TokenPosition::String);
    }

    #[test]
    fn test_scan_marker_in_fstring_expr() {
        let result: Vec<_> = Scanner::new(r#"f"{#{v}}""#).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pos, TokenPosition::Code);
    }

    #[test]
    fn test_scan_escaped_brace_in_fstring() {
        // `{{` is an escaped literal brace, not an expression hole;
        // `#{v}` occurs inside the f-string, so it's returned;
        // `}}` is an escaped literal closing brace.
        let result: Vec<_> = Scanner::new(r#"f"{{#{v}}}""#).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "v");
        assert_eq!(result[0].pos, TokenPosition::String);
    }

    #[test]
    fn test_scan_nested_plain_string_in_fstring_expr_ignored() {
        // `f"{'hello #{v}'}"` is a plain (not interpolated or
        // triple-quoted) string; `#{v}` occurs inside it, so
        // it shouldn't be treated as a placeholder, despite
        // the outer string being an f-string.
        let result: Vec<_> = Scanner::new("f\"{'hello #{v}'}\"").collect();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_scan_marker_nested_fstring_in_fstring_expr() {
        // `f"{f'hello #{v}'}"` is an f-string inside an f-string,
        // so `#{v}` should be treated as a placeholder.
        let result: Vec<_> = Scanner::new(r#"f"{f'hello #{v}'}""#).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "v");
        assert_eq!(result[0].pos, TokenPosition::String);
    }

    #[test]
    fn test_scan_dict_in_fstring_expr() {
        // `f"{ {1: #{v}} }"` is a dict literal inside an f-string,
        // so we should see the placeholder in code position.
        let result: Vec<_> = Scanner::new("f\"{ {1: #{v}} }\"").collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pos, TokenPosition::Code);
    }

    // MARK: Scanner, t-strings

    #[test]
    fn test_scan_marker_in_tstring_text() {
        let result: Vec<_> = Scanner::new(r#"t"hello #{v}""#).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pos, TokenPosition::String);
    }

    #[test]
    fn test_scan_marker_in_tstring_expr() {
        let result: Vec<_> = Scanner::new(r#"t"{#{v}}""#).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pos, TokenPosition::Code);
    }
}
