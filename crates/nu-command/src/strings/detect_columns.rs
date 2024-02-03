use std::iter::Peekable;
use std::str::CharIndices;

use itertools::Itertools;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoInterruptiblePipelineData, PipelineData, Range, Record,
    ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
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
            .named(
                "combine-columns",
                SyntaxShape::Range,
                "columns to be combined; listed as a range",
                Some('c'),
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Attempt to automatically split text into multiple columns."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["split", "tabular"]
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
        vec![
            Example {
                description: "Splits string across multiple columns",
                example: "'a b c' | detect columns --no-headers",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                        "column0" => Value::test_string("a"),
                        "column1" => Value::test_string("b"),
                        "column2" => Value::test_string("c"),
                })])),
            },
            Example {
                description: "",
                example:
                    "$'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns --combine-columns 0..1",
                result: None,
            },
            Example {
                description: "Splits a multi-line string into columns with headers detected",
                example:
                    "$'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns --combine-columns -2..-1",
                result: None,
            },
            Example {
                description: "Splits a multi-line string into columns with headers detected",
                example:
                    "$'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns --combine-columns 2..",
                result: None,
            },
            Example {
                description: "Parse external ls command and combine columns for datetime",
                example: "^ls -lh | detect columns --no-headers --skip 1 --combine-columns 5..7",
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
    let noheader = call.has_flag(engine_state, stack, "no-headers")?;
    let range: Option<Range> = call.get_flag(engine_state, stack, "combine-columns")?;
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
                    vals.push(Value::string(val.item.clone(), name_span));
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

            let (start_index, end_index) = if let Some(range) = &range {
                match nu_cmd_base::util::process_range(range) {
                    Ok((l_idx, r_idx)) => {
                        let l_idx = if l_idx < 0 {
                            cols.len() as isize + l_idx
                        } else {
                            l_idx
                        };

                        let r_idx = if r_idx < 0 {
                            cols.len() as isize + r_idx
                        } else {
                            r_idx
                        };

                        if !(l_idx <= r_idx && (r_idx >= 0 || l_idx < (cols.len() as isize))) {
                            return Value::record(
                                Record::from_raw_cols_vals_unchecked(cols, vals),
                                name_span,
                            );
                        }

                        (l_idx.max(0) as usize, (r_idx as usize + 1).min(cols.len()))
                    }
                    Err(processing_error) => {
                        let err = processing_error("could not find range index", name_span);
                        return Value::error(err, name_span);
                    }
                }
            } else {
                return Value::record(Record::from_raw_cols_vals_unchecked(cols, vals), name_span);
            };

            // Merge Columns
            ((start_index + 1)..(cols.len() - end_index + start_index + 1)).for_each(|idx| {
                cols.swap(idx, end_index - start_index - 1 + idx);
            });
            cols.truncate(cols.len() - end_index + start_index + 1);

            // Merge Values
            let combined = vals
                .iter()
                .take(end_index)
                .skip(start_index)
                .map(|v| v.as_string().unwrap_or_default())
                .join(" ");
            let binding = Value::string(combined, Span::unknown());
            let last_seg = vals.split_off(end_index);
            vals.truncate(start_index);
            vals.push(binding);
            last_seg.into_iter().for_each(|v| vals.push(v));

            Value::record(Record::from_raw_cols_vals_unchecked(cols, vals), name_span)
        })
        .into_pipeline_data(ctrlc))
    } else {
        Ok(PipelineData::empty())
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
    Parenthesis,
    Brace,
    Bracket,
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
            block_level.push(BlockKind::Bracket);
        } else if c == ']' {
            // We encountered a closing `]` delimiter. Pop off the opening `[`
            // delimiter.
            if let Some(BlockKind::Bracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '{' {
            // We encountered an opening `{` delimiter.
            block_level.push(BlockKind::Brace);
        } else if c == '}' {
            // We encountered a closing `}` delimiter. Pop off the opening `{`.
            if let Some(BlockKind::Brace) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '(' {
            // We enceountered an opening `(` delimiter.
            block_level.push(BlockKind::Parenthesis);
        } else if c == ')' {
            // We encountered a closing `)` delimiter. Pop off the opening `(`.
            if let Some(BlockKind::Parenthesis) = block_level.last() {
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
