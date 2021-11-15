use std::{
    iter::Peekable,
    str::{Bytes, CharIndices, Chars},
};

use crate::prelude::*;
use log::trace;
use nu_engine::WholeStreamCommand;
use nu_errors::{ParseError, ShellError};
use nu_protocol::{
    Primitive, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
};
use nu_source::{Spanned, Tagged};

type Input<'t> = Peekable<CharIndices<'t>>;

pub struct DetectColumns;

impl WholeStreamCommand for DetectColumns {
    fn name(&self) -> &str {
        "detect columns"
    }

    fn signature(&self) -> Signature {
        Signature::build("detect columns")
            .named(
                "skip",
                SyntaxShape::Int,
                "number of rows to skip before detecting",
                Some('s'),
            )
            .switch("no_headers", "don't detect headers", Some('n'))
    }

    fn usage(&self) -> &str {
        "splits contents across multiple columns via the separator."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        detect_columns(args)
    }
}

fn detect_columns(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = args.name_tag();
    let num_rows_to_skip: Option<usize> = args.get_flag("skip")?;
    let noheader = args.has_flag("no_headers");
    let input = args.input.collect_string(name_tag.clone())?;

    let input: Vec<_> = input
        .lines()
        .skip(num_rows_to_skip.unwrap_or_default())
        .map(|x| x.to_string())
        .collect();

    let mut input = input.into_iter();
    let headers = input.next();

    if let Some(orig_headers) = headers {
        let headers = find_columns(&orig_headers);

        Ok((if noheader {
            vec![orig_headers].into_iter().chain(input)
        } else {
            vec![].into_iter().chain(input)
        })
        .map(move |x| {
            let row = find_columns(&x);

            let mut dict = TaggedDictBuilder::new(name_tag.clone());

            if headers.len() == row.len() && !noheader {
                for (header, val) in headers.iter().zip(row.iter()) {
                    dict.insert_untagged(&header.item, UntaggedValue::string(&val.item));
                }
            } else {
                let mut pre_output = vec![];

                // column counts don't line up, so see if we can figure out why
                for cell in row {
                    for header in &headers {
                        if cell.span.start() <= header.span.end()
                            && cell.span.end() > header.span.start()
                        {
                            pre_output
                                .push((header.item.to_string(), UntaggedValue::string(&cell.item)));
                        }
                    }
                }

                for header in &headers {
                    let mut found = false;
                    for pre_o in &pre_output {
                        if pre_o.0 == header.item {
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        pre_output.push((header.item.to_string(), UntaggedValue::nothing()));
                    }
                }

                if noheader {
                    for header in headers.iter().enumerate() {
                        for pre_o in &pre_output {
                            if pre_o.0 == header.1.item {
                                dict.insert_untagged(format!("Column{}", header.0), pre_o.1.clone())
                            }
                        }
                    }
                } else {
                    for header in &headers {
                        for pre_o in &pre_output {
                            if pre_o.0 == header.item {
                                dict.insert_untagged(&header.item, pre_o.1.clone())
                            }
                        }
                    }
                }
            }

            dict.into_value()
        })
        .into_output_stream())
    } else {
        Ok(OutputStream::empty())
    }
}

pub fn find_columns(input: &str) -> Vec<Spanned<String>> {
    let mut chars = input.char_indices().peekable();
    let mut output = vec![];

    while let Some((idx, c)) = chars.peek() {
        if c.is_whitespace() {
            // If the next character is non-newline whitespace, skip it.

            let _ = chars.next();
        } else {
            // Otherwise, try to consume an unclassified token.

            let result = baseline(&mut chars);

            output.push(result);
        }
    }

    output
}

#[derive(Clone, Copy)]
enum BlockKind {
    Paren,
    CurlyBracket,
    SquareBracket,
}

impl BlockKind {
    fn closing(self) -> char {
        match self {
            BlockKind::Paren => ')',
            BlockKind::SquareBracket => ']',
            BlockKind::CurlyBracket => '}',
        }
    }
}

fn baseline(src: &mut Input) -> Spanned<String> {
    let mut token_contents = String::new();

    let start_offset = if let Some((pos, _)) = src.peek() {
        *pos
    } else {
        0
    };

    // This variable tracks the starting character of a string literal, so that
    // we remain inside the string literal lexer mode until we encounter the
    // closing quote.
    let mut quote_start: Option<char> = None;

    let mut in_comment = false;

    // This Vec tracks paired delimiters
    let mut block_level: Vec<BlockKind> = vec![];

    // A baseline token is terminated if it's not nested inside of a paired
    // delimiter and the next character is one of: `|`, `;`, `#` or any
    // whitespace.
    fn is_termination(block_level: &[BlockKind], c: char) -> bool {
        block_level.is_empty() && (c.is_whitespace())
    }

    // The process of slurping up a baseline token repeats:
    //
    // - String literal, which begins with `'`, `"` or `\``, and continues until
    //   the same character is encountered again.
    // - Delimiter pair, which begins with `[`, `(`, or `{`, and continues until
    //   the matching closing delimiter is found, skipping comments and string
    //   literals.
    // - When not nested inside of a delimiter pair, when a terminating
    //   character (whitespace, `|`, `;` or `#`) is encountered, the baseline
    //   token is done.
    // - Otherwise, accumulate the character into the current baseline token.
    while let Some((idx, c)) = src.peek() {
        let c = *c;

        if quote_start.is_some() {
            // If we encountered the closing quote character for the current
            // string, we're done with the current string.
            if Some(c) == quote_start {
                quote_start = None;
            }
        } else if c == '\n' {
            in_comment = false;
            if is_termination(&block_level, c) {
                break;
            }
        } else if c == '\'' || c == '"' || c == '`' {
            // We encountered the opening quote of a string literal.
            quote_start = Some(c);
        } else if c == '[' {
            // We encountered an opening `[` delimiter.
            block_level.push(BlockKind::SquareBracket);
        } else if c == ']' {
            // We encountered a closing `]` delimiter. Pop off the opening `[`
            // delimiter.
            if let Some(BlockKind::SquareBracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '{' {
            // We encountered an opening `{` delimiter.
            block_level.push(BlockKind::CurlyBracket);
        } else if c == '}' {
            // We encountered a closing `}` delimiter. Pop off the opening `{`.
            if let Some(BlockKind::CurlyBracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '(' {
            // We enceountered an opening `(` delimiter.
            block_level.push(BlockKind::Paren);
        } else if c == ')' {
            // We encountered a closing `)` delimiter. Pop off the opening `(`.
            if let Some(BlockKind::Paren) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if is_termination(&block_level, c) {
            break;
        }

        // Otherwise, accumulate the character into the current token.
        token_contents.push(c);

        // Consume the character.
        let _ = src.next();
    }

    let span = Span::new(start_offset, start_offset + token_contents.len());

    // If there is still unclosed opening delimiters, close them and add
    // synthetic closing characters to the accumulated token.
    if let Some(block) = block_level.last() {
        // let delim: char = (*block).closing();
        // let cause = ParseError::unexpected_eof(delim.to_string(), span);

        // while let Some(bk) = block_level.pop() {
        //     token_contents.push(bk.closing());
        // }

        return token_contents.spanned(span);
    }

    if let Some(delimiter) = quote_start {
        // The non-lite parse trims quotes on both sides, so we add the expected quote so that
        // anyone wanting to consume this partial parse (e.g., completions) will be able to get
        // correct information from the non-lite parse.
        // token_contents.push(delimiter);

        // return (
        //     token_contents.spanned(span),
        //     Some(ParseError::unexpected_eof(delimiter.to_string(), span)),
        // );
        return token_contents.spanned(span);
    }

    token_contents.spanned(span)
}

#[cfg(test)]
mod tests {
    use super::DetectColumns;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(DetectColumns {})
    }
}
