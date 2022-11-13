use std::iter::Peekable;
use std::str::CharIndices;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};

type Input<'t> = Peekable<CharIndices<'t>>;

#[derive(Clone)]
pub struct DetectColumns;

impl Command for DetectColumns {
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
            .input_output_types(vec![(Type::String, Type::Table(vec![]))])
            .switch("no-headers", "don't detect headers", Some('n'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Attempt to automatically split text into multiple columns"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["split"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        detect_columns(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Splits string across multiple columns",
                example: "echo 'a b c' | detect columns -n",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec![
                            "column0".to_string(),
                            "column1".to_string(),
                            "column2".to_string(),
                        ],
                        vals: vec![
                            Value::test_string("a"),
                            Value::test_string("b"),
                            Value::test_string("c"),
                        ],
                        span,
                    }],
                    span,
                }),
            },
            Example {
                description: "Splits a multi-line string into columns with headers detected",
                example: "echo $'c1 c2 c3(char nl)a b c' | detect columns",
                result: None,
            },
        ]
    }
}

fn detect_columns(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name_span = call.head;
    let num_rows_to_skip: Option<usize> = call.get_flag(engine_state, stack, "skip")?;
    let noheader = call.has_flag("no-headers");
    let ctrlc = engine_state.ctrlc.clone();
    let config = engine_state.get_config();
    let input = input.collect_string("", config)?;

    #[allow(clippy::needless_collect)]
    let input: Vec<_> = input
        .lines()
        .skip(num_rows_to_skip.unwrap_or_default())
        .map(|x| x.to_string())
        .collect();

    let mut input = input.into_iter();
    let headers = input.next();

    if let Some(orig_headers) = headers {
        let mut headers = find_columns(&orig_headers);

        if noheader {
            for header in headers.iter_mut().enumerate() {
                header.1.item = format!("column{}", header.0);
            }
        }

        Ok((if noheader {
            vec![orig_headers].into_iter().chain(input)
        } else {
            vec![].into_iter().chain(input)
        })
        .map(move |x| {
            let row = find_columns(&x);

            let mut cols = vec![];
            let mut vals = vec![];

            if headers.len() == row.len() {
                for (header, val) in headers.iter().zip(row.iter()) {
                    cols.push(header.item.clone());
                    vals.push(Value::String {
                        val: val.item.clone(),
                        span: name_span,
                    });
                }
            } else {
                let mut pre_output = vec![];

                // column counts don't line up, so see if we can figure out why
                for cell in row {
                    for header in &headers {
                        if cell.span.start <= header.span.end && cell.span.end > header.span.start {
                            pre_output.push((
                                header.item.to_string(),
                                Value::string(&cell.item, name_span),
                            ));
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
                        pre_output.push((header.item.to_string(), Value::nothing(name_span)));
                    }
                }

                for header in &headers {
                    for pre_o in &pre_output {
                        if pre_o.0 == header.item {
                            cols.push(header.item.clone());
                            vals.push(pre_o.1.clone())
                        }
                    }
                }
            }

            Value::Record {
                cols,
                vals,
                span: name_span,
            }
        })
        .into_pipeline_data(ctrlc))
    } else {
        Ok(PipelineData::new(name_span))
    }
}

pub fn find_columns(input: &str) -> Vec<Spanned<String>> {
    let mut chars = input.char_indices().peekable();
    let mut output = vec![];

    while let Some((_, c)) = chars.peek() {
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
    while let Some((_, c)) = src.peek() {
        let c = *c;

        if quote_start.is_some() {
            // If we encountered the closing quote character for the current
            // string, we're done with the current string.
            if Some(c) == quote_start {
                quote_start = None;
            }
        } else if c == '\n' {
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
    if block_level.last().is_some() {
        // let delim: char = (*block).closing();
        // let cause = ParseError::unexpected_eof(delim.to_string(), span);

        // while let Some(bk) = block_level.pop() {
        //     token_contents.push(bk.closing());
        // }

        return Spanned {
            item: token_contents,
            span,
        };
    }

    if quote_start.is_some() {
        // The non-lite parse trims quotes on both sides, so we add the expected quote so that
        // anyone wanting to consume this partial parse (e.g., completions) will be able to get
        // correct information from the non-lite parse.
        // token_contents.push(delimiter);

        // return (
        //     token_contents.spanned(span),
        //     Some(ParseError::unexpected_eof(delimiter.to_string(), span)),
        // );
        return Spanned {
            item: token_contents,
            span,
        };
    }

    Spanned {
        item: token_contents,
        span,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(DetectColumns)
    }
}
