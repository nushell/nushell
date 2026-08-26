use nu_protocol::{ParseError, Span};

#[path = "delimiter_diagnostics.rs"]
mod delimiter_diagnostics;
use delimiter_diagnostics::{
    closing_delimiter_str, quote_delimiter_str, unbalanced_closer, unclosed_from_open,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenContents {
    Item,
    Comment,
    Pipe,
    PipePipe,
    AssignmentOperator,
    ErrGreaterPipe,
    OutErrGreaterPipe,
    Semicolon,
    OutGreaterThan,
    OutGreaterGreaterThan,
    ErrGreaterThan,
    ErrGreaterGreaterThan,
    OutErrGreaterThan,
    OutErrGreaterGreaterThan,
    Eol,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub contents: TokenContents,
    pub span: Span,
}

impl Token {
    pub fn new(contents: TokenContents, span: Span) -> Token {
        Token { contents, span }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BlockKind {
    Paren,
    CurlyBracket,
    SquareBracket,
    AngleBracket,
}

/// An open delimiter on the lexer's nesting stack (kind + opener span only).
///
/// Opener spans are used only to *label* a real unclosed/unbalanced error for
/// miette. Indent/structure heuristics must never invent a parse failure — the
/// stack alone decides whether lexing failed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OpenFrame {
    pub kind: BlockKind,
    pub open_span: Span,
}

// A baseline token is terminated if it's not nested inside of a paired
// delimiter and the next character is one of: `|`, `;` or any
// whitespace.
fn is_item_terminator(
    block_level: &[OpenFrame],
    c: u8,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
) -> bool {
    block_level.is_empty()
        && (c == b' '
            || c == b'\t'
            || c == b'\n'
            || c == b'\r'
            || c == b'|'
            || c == b';'
            || additional_whitespace.contains(&c)
            || special_tokens.contains(&c))
}

/// Assignment operators have special handling distinct from math expressions, as they cause the
/// rest of the pipeline to be consumed.
pub fn is_assignment_operator(bytes: &[u8]) -> bool {
    matches!(bytes, b"=" | b"+=" | b"++=" | b"-=" | b"*=" | b"/=")
}

// A special token is one that is a byte that stands alone as its own token. For example
// when parsing a signature you may want to have `:` be able to separate tokens and also
// to be handled as its own token to notify you you're about to parse a type in the example
// `foo:bar`
fn is_special_item(block_level: &[OpenFrame], c: u8, special_tokens: &[u8]) -> bool {
    block_level.is_empty() && special_tokens.contains(&c)
}

/// A better place to put the "expected closer" miette label when the real stack
/// failure is only known at end-of-token (often far from the human mistake).
///
/// Never used to invent an error — only to choose spans for an error that
/// already exists because `block_level` is non-empty at the end.
#[derive(Clone, Copy, Debug)]
struct CloserLabelHint {
    /// Opener most likely related to the missing closer (often an inner `{|…`).
    open_span: Span,
    /// Where the missing closer probably belongs.
    expected_span: Span,
}

/// True if `c` can legally continue a multi-line construct onto the next line
/// (so a following line starting with `|` is not a missing-`}` signal).
fn continues_onto_next_line(c: u8) -> bool {
    matches!(
        c,
        b'|' | b'{' | b'(' | b'[' | b',' | b':' | b'+' | b'-' | b'*' | b'/' | b'=' | b'.'
    )
}

/// Advance the delimiter matching for one byte inside a subexpression of an
/// interpolated string. Shared by `lex_item` and `parse_string_interpolation`
/// so both scan the same bytes the same way. The stack holds expected closers;
/// `open` is stored alongside a pushed closer (a span for error reporting, or
/// `()` when the caller does not need one).
///
/// While the innermost open delimiter is a quote, only that quote closes it;
/// otherwise quotes open nested strings and parens nest. Escapes exist only in
/// double-quoted strings: returns true when `byte` is a backslash inside a
/// nested `"` string, in which case the caller must also skip the next byte.
pub(crate) fn interp_subexpr_step<T>(stack: &mut Vec<(u8, T)>, byte: u8, open: T) -> bool {
    match stack.last() {
        Some(&(expected, _)) if expected != b')' => {
            if expected == b'"' && byte == b'\\' {
                return true;
            }
            if byte == expected {
                stack.pop();
            }
        }
        _ => match byte {
            b'\'' | b'"' | b'`' => stack.push((byte, open)),
            b'(' => stack.push((b')', open)),
            b')' => {
                stack.pop();
            }
            _ => {}
        },
    }
    false
}

pub fn lex_item(
    input: &[u8],
    curr_offset: &mut usize,
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    in_signature: bool,
) -> (Token, Option<ParseError>) {
    // Tracks the opening quote character and its span while inside a string.
    let mut quote_start: Option<(u8, Span)> = None;

    // True while the current string is an interpolated one (its opening quote
    // directly follows `$`). Inside such a string an unescaped `(` starts a
    // subexpression, where quotes and parens nest.
    let mut quote_is_interp = false;

    // Expected closers (with opener spans) while inside a subexpression of an
    // interpolated string. Non-empty means the string's own closing quote does
    // not end it yet. Mirrors the delimiter matching that
    // `parse_string_interpolation` later applies to the same bytes, so the
    // token ends exactly where the parser will end the string.
    let mut interp_expr_level: Vec<(u8, Span)> = vec![];

    let mut in_comment = false;

    let token_start = *curr_offset;

    // Paired delimiters with opener spans (for labeling real unclosed errors only).
    let mut block_level: Vec<OpenFrame> = vec![];

    // Presentation-only: first place a missing `}` may belong (e.g. before a
    // pipeline step that should have been outside a closure). Used solely when
    // the stack still has openers at end-of-token — never to invent failures.
    let mut closer_label_hint: Option<CloserLabelHint> = None;

    // Line tracking for the presentation hint above (not for inventing errors).
    let mut at_line_start = true;
    // Last non-whitespace, non-comment char on the previous line (if any).
    let mut prev_line_continue = false;
    let mut last_sig_char: Option<u8> = None;

    // The process of slurping up a baseline token repeats:
    //
    // - String literal, which begins with `'` or `"`, and continues until
    //   the same character is encountered again.
    // - Delimiter pair, which begins with `[`, `(`, or `{`, and continues until
    //   the matching closing delimiter is found, skipping comments and string
    //   literals.
    // - When not nested inside of a delimiter pair, when a terminating
    //   character (whitespace, `|`, `;` or `#`) is encountered, the baseline
    //   token is done.
    // - Otherwise, accumulate the character into the current baseline token.
    //
    // Parse *failure* is decided only by the delimiter stack / quotes — never by
    // line-shape heuristics. Heuristics may only choose spans/help when a real
    // failure is reported.
    let mut previous_char = None;
    while let Some(c) = input.get(*curr_offset) {
        let c = *c;

        if let Some((start, open_span)) = quote_start {
            if !interp_expr_level.is_empty() {
                // Inside a subexpression of an interpolated string; the shared
                // step keeps this scan and `parse_string_interpolation` on the
                // same rules, so the token ends where the parser ends the
                // string.
                let open = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
                if interp_subexpr_step(&mut interp_expr_level, c, open)
                    && input.get(*curr_offset + 1).is_some()
                {
                    // Escape inside a nested double-quoted string: consume the
                    // escaped byte too, so `\"` does not close the string.
                    *curr_offset += 2;
                    previous_char = Some(c);
                    at_line_start = false;
                    continue;
                }
                last_sig_char = Some(c);
                at_line_start = false;
                *curr_offset += 1;
                previous_char = Some(c);
                continue;
            }
            // Check if we're in an escape sequence
            if c == b'\\' && start == b'"' {
                // Go ahead and consume the escape character if possible
                if input.get(*curr_offset + 1).is_some() {
                    // Successfully escaped the character
                    *curr_offset += 2;
                    previous_char = Some(c);
                    at_line_start = false;
                    continue;
                } else {
                    let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
                    let end_span = if span.end > span.start {
                        Span::new(span.end - 1, span.end)
                    } else {
                        span
                    };

                    return (
                        Token {
                            contents: TokenContents::Item,
                            span,
                        },
                        Some(unclosed_from_open(
                            input,
                            span_offset,
                            quote_delimiter_str(start),
                            open_span,
                            end_span,
                        )),
                    );
                }
            }
            // If we encountered the closing quote character for the current
            // string, we're done with the current string.
            if c == start {
                // Also need to check to make sure we aren't escaped
                quote_start = None;
            } else if quote_is_interp && c == b'(' {
                // An unescaped `(` in an interpolated string starts a
                // subexpression (an escaped one was already consumed by the
                // escape handling above). The string's closing quote cannot
                // end it until the matching `)` is found.
                interp_expr_level.push((
                    b')',
                    Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1),
                ));
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'#' && !in_comment {
            // To start a comment, It either need to be the first character of the token or prefixed with whitespace.
            in_comment = previous_char
                .map(char::from)
                .map(char::is_whitespace)
                .unwrap_or(true);
        } else if c == b'\n' || c == b'\r' {
            in_comment = false;
            if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
                break;
            }
            // Commit previous line's trailing significant char for next-line `|` hints.
            // For `\r\n`, only commit/reset on `\n` so we don't double-reset.
            let is_newline_end = c == b'\n' || input.get(*curr_offset + 1) != Some(&b'\n');
            if is_newline_end {
                prev_line_continue = last_sig_char.is_some_and(continues_onto_next_line);
                at_line_start = true;
                last_sig_char = None;
            }
        } else if in_comment {
            if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
                break;
            }
        } else if is_special_item(&block_level, c, special_tokens) && token_start == *curr_offset {
            *curr_offset += 1;
            break;
        } else if c == b'\'' || c == b'"' || c == b'`' {
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            quote_start = Some((c, open_span));
            // `$"` and `$'` open interpolated strings, where `(` starts a
            // subexpression. Backtick strings never interpolate.
            quote_is_interp = c != b'`' && previous_char == Some(b'$');
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'[' {
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            block_level.push(OpenFrame {
                kind: BlockKind::SquareBracket,
                open_span,
            });
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'<' && in_signature {
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            block_level.push(OpenFrame {
                kind: BlockKind::AngleBracket,
                open_span,
            });
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'>' && in_signature {
            if let Some(OpenFrame {
                kind: BlockKind::AngleBracket,
                ..
            }) = block_level.last()
            {
                let _ = block_level.pop();
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b']' {
            // Closing `]` — pop matching `[`, else real mismatch if another opener is open.
            if let Some(OpenFrame {
                kind: BlockKind::SquareBracket,
                ..
            }) = block_level.last()
            {
                let _ = block_level.pop();
            } else if !block_level.is_empty() {
                *curr_offset += 1;
                let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
                let close_span = Span::new(span.end - 1, span.end);
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(unbalanced_closer("]", "[", &block_level, close_span)),
                );
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'{' {
            // Presentation only: `def name [\n  param\n {` without `]` — the body
            // `{` is where `]` should have been. Record for labeling if the `[`
            // is still open at end-of-token (real stack failure).
            if closer_label_hint.is_none()
                && let Some(frame) = block_level.last()
                && matches!(frame.kind, BlockKind::SquareBracket)
            {
                closer_label_hint = Some(CloserLabelHint {
                    open_span: frame.open_span,
                    expected_span: Span::new(
                        span_offset + *curr_offset,
                        span_offset + *curr_offset + 1,
                    ),
                });
            }
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            block_level.push(OpenFrame {
                kind: BlockKind::CurlyBracket,
                open_span,
            });
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'}' {
            // Closing `}` — pop matching `{`, else real mismatch against stack top.
            if let Some(OpenFrame {
                kind: BlockKind::CurlyBracket,
                ..
            }) = block_level.last()
            {
                let _ = block_level.pop();
            } else {
                *curr_offset += 1;
                let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
                let close_span = Span::new(span.end - 1, span.end);
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(unbalanced_closer("}", "{", &block_level, close_span)),
                );
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'(' {
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            block_level.push(OpenFrame {
                kind: BlockKind::Paren,
                open_span,
            });
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b')' {
            // Closing `)` — pop matching `(`, else real mismatch against stack top.
            if let Some(OpenFrame {
                kind: BlockKind::Paren,
                ..
            }) = block_level.last()
            {
                let _ = block_level.pop();
            } else {
                *curr_offset += 1;
                let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
                let close_span = Span::new(span.end - 1, span.end);
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(unbalanced_closer(")", "(", &block_level, close_span)),
                );
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'r' && input.get(*curr_offset + 1) == Some(b'#').as_ref() {
            // already checked `r#` pattern, so it's a raw string.
            let lex_result = lex_raw_string(input, curr_offset, span_offset);
            let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
            if let Err(e) = lex_result {
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(e),
                );
            }
            last_sig_char = Some(b'#');
            at_line_start = false;
        } else if c == b'|' && is_redirection(&input[token_start..*curr_offset]) {
            // matches err>| etc.
            *curr_offset += 1;
            break;
        } else if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
            break;
        } else if !c.is_ascii_whitespace() {
            // Presentation hint only: a new line starting with `|` while nested
            // in `{…}`, when the previous line did not end with a continue char,
            // often means a missing `}` before this pipeline step (e.g. forgot
            // to close `{|n| … }` before `| upsert …`).
            //
            // We only *record* this; an error is emitted only if the stack is
            // still non-empty at end-of-token.
            if c == b'|'
                && at_line_start
                && !prev_line_continue
                && closer_label_hint.is_none()
                && let Some(frame) = block_level
                    .iter()
                    .rev()
                    .find(|f| matches!(f.kind, BlockKind::CurlyBracket))
            {
                closer_label_hint = Some(CloserLabelHint {
                    open_span: frame.open_span,
                    expected_span: Span::new(
                        span_offset + *curr_offset,
                        span_offset + *curr_offset + 1,
                    ),
                });
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if at_line_start && (c == b' ' || c == b'\t') {
            // stay at line start until real content
        } else {
            at_line_start = false;
        }

        *curr_offset += 1;
        previous_char = Some(c);
    }

    let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
    let end_span = if span.end > span.start {
        Span::new(span.end - 1, span.end)
    } else {
        span
    };

    // An open delimiter inside an interpolated string's subexpression is more
    // precise than the enclosing quote. Report the oldest one: in the common
    // `$"foo (2 + 3"` typo the trailing quote was meant to close the string,
    // and the actual mistake is the unclosed `(`.
    if let Some((closer, open_span)) = interp_expr_level.first() {
        let closer_str = match closer {
            b')' => ")",
            delim => quote_delimiter_str(*delim),
        };
        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(unclosed_from_open(
                input,
                span_offset,
                closer_str,
                *open_span,
                end_span,
            )),
        );
    }

    if let Some((delim, open_span)) = quote_start {
        // The non-lite parse trims quotes on both sides, so we add the expected quote so that
        // anyone wanting to consume this partial parse (e.g., completions) will be able to get
        // correct information from the non-lite parse.
        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(unclosed_from_open(
                input,
                span_offset,
                quote_delimiter_str(delim),
                open_span,
                end_span,
            )),
        );
    }

    // Still-unclosed openers at end of token: real stack failure.
    // Prefer a recorded closer-label hint when it refers to the *same* open frame
    // still on the stack (presentation only — error already exists).
    if let Some(frame) = block_level.last() {
        let (label_open, label_end) = closer_label_hint
            .filter(|h| h.open_span == frame.open_span)
            .map(|h| (h.open_span, h.expected_span))
            .unwrap_or((frame.open_span, end_span));

        let cause = unclosed_from_open(
            input,
            span_offset,
            closing_delimiter_str(frame.kind),
            label_open,
            label_end,
        );

        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(cause),
        );
    }

    // If we didn't accumulate any characters, it's an unexpected error.
    if *curr_offset - token_start == 0 {
        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(ParseError::UnexpectedEof("command".to_string(), span)),
        );
    }

    let mut err = None;
    let output = match &input[(span.start - span_offset)..(span.end - span_offset)] {
        bytes if is_assignment_operator(bytes) => Token {
            contents: TokenContents::AssignmentOperator,
            span,
        },
        b"out>" | b"o>" => Token {
            contents: TokenContents::OutGreaterThan,
            span,
        },
        b"out>>" | b"o>>" => Token {
            contents: TokenContents::OutGreaterGreaterThan,
            span,
        },
        b"out>|" | b"o>|" => {
            err = Some(ParseError::Expected(
                "`|`.  Redirecting stdout to a pipe is the same as normal piping.",
                span,
            ));
            Token {
                // HACK: For more accurate parsing aligned with user intention
                contents: TokenContents::Pipe,
                span,
            }
        }
        b"err>" | b"e>" => Token {
            contents: TokenContents::ErrGreaterThan,
            span,
        },
        b"err>>" | b"e>>" => Token {
            contents: TokenContents::ErrGreaterGreaterThan,
            span,
        },
        b"err>|" | b"e>|" => Token {
            contents: TokenContents::ErrGreaterPipe,
            span,
        },
        b"out+err>" | b"err+out>" | b"o+e>" | b"e+o>" => Token {
            contents: TokenContents::OutErrGreaterThan,
            span,
        },
        b"out+err>>" | b"err+out>>" | b"o+e>>" | b"e+o>>" => Token {
            contents: TokenContents::OutErrGreaterGreaterThan,
            span,
        },
        b"out+err>|" | b"err+out>|" | b"o+e>|" | b"e+o>|" => Token {
            contents: TokenContents::OutErrGreaterPipe,
            span,
        },
        b"&&" => {
            err = Some(ParseError::ShellAndAnd(span));
            Token {
                // HACK: For more accurate parsing aligned with user intention
                contents: TokenContents::Pipe,
                span,
            }
        }
        b"2>" => {
            err = Some(ParseError::ShellErrRedirect(span));
            Token {
                // HACK: For more accurate parsing aligned with user intention
                contents: TokenContents::ErrGreaterThan,
                span,
            }
        }
        b"2>&1" => {
            err = Some(ParseError::ShellOutErrRedirect(span));
            Token {
                // HACK: For more accurate parsing aligned with user intention
                contents: TokenContents::Pipe,
                span,
            }
        }
        _ => Token {
            contents: TokenContents::Item,
            span,
        },
    };
    (output, err)
}

fn lex_raw_string(
    input: &[u8],
    curr_offset: &mut usize,
    span_offset: usize,
) -> Result<(), ParseError> {
    // A raw string literal looks like `echo r#'Look, I can use 'single quotes'!'#`
    // If the next character is `#` we're probably looking at a raw string literal
    // so we need to read all the text until we find a closing `#`. This raw string
    // can contain any character, including newlines and double quotes without needing
    // to escape them.
    //
    // A raw string can contain many `#` as prefix,
    // incase if there is a `'#` or `#'` in the string itself.
    // E.g: r##'I can use '#' in a raw string'##
    let mut prefix_sharp_cnt = 0;
    let start = *curr_offset;
    while let Some(b'#') = input.get(start + prefix_sharp_cnt + 1) {
        prefix_sharp_cnt += 1;
    }

    // curr_offset is the character `r`, we need to move forward and skip all `#`
    // characters.
    //
    // e.g: r###'<body>
    //      ^
    //      ^
    //   curr_offset
    *curr_offset += prefix_sharp_cnt + 1;
    // the next one should be a single quote.
    if input.get(*curr_offset) != Some(&b'\'') {
        return Err(ParseError::Expected(
            "'",
            Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1),
        ));
    }

    *curr_offset += 1;
    let mut matches = false;
    while let Some(ch) = input.get(*curr_offset) {
        // check for postfix '###
        if *ch == b'#' {
            let start_ch = input[*curr_offset - prefix_sharp_cnt];
            let postfix = &input[*curr_offset - prefix_sharp_cnt + 1..=*curr_offset];
            if start_ch == b'\'' && postfix.iter().all(|x| *x == b'#') {
                matches = true;
                break;
            }
        }
        *curr_offset += 1
    }
    if !matches {
        let mut expected = '\''.to_string();
        expected.push_str(&"#".repeat(prefix_sharp_cnt));
        return Err(ParseError::UnexpectedEof(
            expected,
            Span::new(span_offset + *curr_offset - 1, span_offset + *curr_offset),
        ));
    }
    Ok(())
}

pub fn lex_signature(
    input: &[u8],
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
) -> (Vec<Token>, Option<ParseError>) {
    let mut state = LexState {
        input,
        output: Vec::new(),
        error: None,
        span_offset,
    };
    lex_internal(
        &mut state,
        additional_whitespace,
        special_tokens,
        skip_comment,
        true,
        None,
    );
    (state.output, state.error)
}

#[derive(Debug)]
pub struct LexState<'a> {
    pub input: &'a [u8],
    pub output: Vec<Token>,
    pub error: Option<ParseError>,
    pub span_offset: usize,
}

/// Lex until the output is `max_tokens` longer than before the call, or until the input is exhausted.
/// The return value indicates how many tokens the call added to / removed from the output.
///
/// The behaviour here is non-obvious when `additional_whitespace` doesn't include newline:
/// If you pass a `state` where the last token in the output is an Eol, this might *remove* tokens.
pub fn lex_n_tokens(
    state: &mut LexState,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
    max_tokens: usize,
) -> isize {
    let n_tokens = state.output.len();
    lex_internal(
        state,
        additional_whitespace,
        special_tokens,
        skip_comment,
        false,
        Some(max_tokens),
    );
    // If this lex_internal call reached the end of the input, there may now be fewer tokens
    // in the output than before.
    let tokens_n_diff = (state.output.len() as isize) - (n_tokens as isize);
    let next_offset = state.output.last().map(|token| token.span.end);
    if let Some(next_offset) = next_offset {
        state.input = &state.input[next_offset - state.span_offset..];
        state.span_offset = next_offset;
    }
    tokens_n_diff
}

pub fn lex(
    input: &[u8],
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
) -> (Vec<Token>, Option<ParseError>) {
    let mut state = LexState {
        input,
        output: Vec::new(),
        error: None,
        span_offset,
    };
    lex_internal(
        &mut state,
        additional_whitespace,
        special_tokens,
        skip_comment,
        false,
        None,
    );
    (state.output, state.error)
}

fn lex_internal(
    state: &mut LexState,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
    // within signatures we want to treat `<` and `>` specially
    in_signature: bool,
    max_tokens: Option<usize>,
) {
    let initial_output_len = state.output.len();

    let mut curr_offset = 0;

    let mut is_complete = true;
    while let Some(c) = state.input.get(curr_offset) {
        if max_tokens
            .is_some_and(|max_tokens| state.output.len() >= initial_output_len + max_tokens)
        {
            break;
        }
        let c = *c;
        if c == b'|' {
            // If the next character is `|`, it's either `|` or `||`.
            let idx = curr_offset;
            let prev_idx = idx;
            curr_offset += 1;

            // If the next character is `|`, we're looking at a `||`.
            if let Some(c) = state.input.get(curr_offset)
                && *c == b'|'
            {
                let idx = curr_offset;
                curr_offset += 1;
                state.output.push(Token::new(
                    TokenContents::PipePipe,
                    Span::new(state.span_offset + prev_idx, state.span_offset + idx + 1),
                ));
                continue;
            }

            // Otherwise, it's just a regular `|` token.

            // Before we push, check to see if the previous character was a newline.
            // If so, then this is a continuation of the previous line
            if let Some(prev) = state.output.last_mut() {
                match prev.contents {
                    TokenContents::Eol => {
                        *prev = Token::new(
                            TokenContents::Pipe,
                            Span::new(state.span_offset + idx, state.span_offset + idx + 1),
                        );
                        // And this is a continuation of the previous line if previous line is a
                        // comment line (combined with EOL + Comment)
                        //
                        // Initially, the last one token is TokenContents::Pipe, we don't need to
                        // check it, so the beginning offset is 2.
                        let mut offset = 2;
                        while state.output.len() > offset {
                            let index = state.output.len() - offset;
                            if state.output[index].contents == TokenContents::Comment
                                && state.output[index - 1].contents == TokenContents::Eol
                            {
                                state.output.remove(index - 1);
                                offset += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    _ => {
                        state.output.push(Token::new(
                            TokenContents::Pipe,
                            Span::new(state.span_offset + idx, state.span_offset + idx + 1),
                        ));
                    }
                }
            } else {
                state.output.push(Token::new(
                    TokenContents::Pipe,
                    Span::new(state.span_offset + idx, state.span_offset + idx + 1),
                ));
            }

            is_complete = false;
        } else if c == b';' {
            // If the next character is a `;`, we're looking at a semicolon token.

            if !is_complete && state.error.is_none() {
                state.error = Some(ParseError::ExtraTokens(Span::new(
                    curr_offset,
                    curr_offset + 1,
                )));
            }
            let idx = curr_offset;
            curr_offset += 1;
            state.output.push(Token::new(
                TokenContents::Semicolon,
                Span::new(state.span_offset + idx, state.span_offset + idx + 1),
            ));
        } else if c == b'\r' {
            // Ignore a stand-alone carriage return
            curr_offset += 1;
        } else if c == b'\n' {
            // If the next character is a newline, we're looking at an EOL (end of line) token.
            let idx = curr_offset;
            curr_offset += 1;
            if !additional_whitespace.contains(&c) {
                state.output.push(Token::new(
                    TokenContents::Eol,
                    Span::new(state.span_offset + idx, state.span_offset + idx + 1),
                ));
            }
        } else if c == b'#' {
            // If the next character is `#`, we're at the beginning of a line
            // comment. The comment continues until the next newline.
            let mut start = curr_offset;

            while let Some(input) = state.input.get(curr_offset) {
                if *input == b'\n' {
                    if !skip_comment {
                        state.output.push(Token::new(
                            TokenContents::Comment,
                            Span::new(state.span_offset + start, state.span_offset + curr_offset),
                        ));
                    }
                    start = curr_offset;

                    break;
                } else {
                    curr_offset += 1;
                }
            }
            if start != curr_offset && !skip_comment {
                state.output.push(Token::new(
                    TokenContents::Comment,
                    Span::new(state.span_offset + start, state.span_offset + curr_offset),
                ));
            }
        } else if c == b' ' || c == b'\t' || additional_whitespace.contains(&c) {
            // If the next character is non-newline whitespace, skip it.
            curr_offset += 1;
        } else {
            let (token, err) = lex_item(
                state.input,
                &mut curr_offset,
                state.span_offset,
                additional_whitespace,
                special_tokens,
                in_signature,
            );
            if state.error.is_none() {
                state.error = err;
            }
            is_complete = true;
            state.output.push(token);
        }
    }
}

/// True if this the start of a redirection. Does not match `>>` or `>|` forms.
fn is_redirection(token: &[u8]) -> bool {
    matches!(
        token,
        b"o>" | b"out>" | b"e>" | b"err>" | b"o+e>" | b"e+o>" | b"out+err>" | b"err+out>"
    )
}
