use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::{Config, Range};
use std::{io::Cursor, iter::Peekable, str::CharIndices, sync::Arc};

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
            .input_output_types(vec![(Type::String, Type::table())])
            .switch("no-headers", "don't detect headers", Some('n'))
            .named(
                "combine-columns",
                SyntaxShape::Range,
                "columns to be combined; listed as a range",
                Some('c'),
            )
            .switch(
                "guess",
                "detect columns by guessing width, it may be useful if default one doesn't work",
                None,
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Attempt to automatically split text into multiple columns."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["split", "tabular"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "use --guess if you find default algorithm not working",
                example: r"
'Filesystem     1K-blocks      Used Available Use% Mounted on
none             8150224         4   8150220   1% /mnt/c' | detect columns --guess",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "Filesystem" => Value::test_string("none"),
                    "1K-blocks" => Value::test_string("8150224"),
                    "Used" => Value::test_string("4"),
                    "Available" => Value::test_string("8150220"),
                    "Use%" => Value::test_string("1%"),
                    "Mounted on" => Value::test_string("/mnt/c")
                })])),
            },
            Example {
                description: "detect columns with no headers",
                example: "'a b c' | detect columns  --no-headers",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                        "column0" => Value::test_string("a"),
                        "column1" => Value::test_string("b"),
                        "column2" => Value::test_string("c"),
                })])),
            },
            Example {
                description: "",
                example: "$'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns --combine-columns 0..1 ",
                result: None,
            },
            Example {
                description: "Splits a multi-line string into columns with headers detected",
                example: "$'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns --combine-columns -2..-1 ",
                result: None,
            },
            Example {
                description: "Splits a multi-line string into columns with headers detected",
                example: "$'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns --combine-columns 2.. ",
                result: None,
            },
            Example {
                description: "Parse external ls command and combine columns for datetime",
                example: "^ls -lh | detect columns --no-headers --skip 1 --combine-columns 5..7",
                result: None,
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let num_rows_to_skip: Option<usize> = call.get_flag(engine_state, stack, "skip")?;
        let noheader = call.has_flag(engine_state, stack, "no-headers")?;
        let range: Option<Range> = call.get_flag(engine_state, stack, "combine-columns")?;
        let config = stack.get_config(engine_state);

        let args = Arguments {
            noheader,
            num_rows_to_skip,
            range,
            config,
        };

        if call.has_flag(engine_state, stack, "guess")? {
            guess_width(engine_state, call, input, args)
        } else {
            detect_columns(engine_state, call, input, args)
        }
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let num_rows_to_skip: Option<usize> = call.get_flag_const(working_set, "skip")?;
        let noheader = call.has_flag_const(working_set, "no-headers")?;
        let range: Option<Range> = call.get_flag_const(working_set, "combine-columns")?;
        let config = working_set.get_config().clone();

        let args = Arguments {
            noheader,
            num_rows_to_skip,
            range,
            config,
        };

        if call.has_flag_const(working_set, "guess")? {
            guess_width(working_set.permanent(), call, input, args)
        } else {
            detect_columns(working_set.permanent(), call, input, args)
        }
    }
}

struct Arguments {
    num_rows_to_skip: Option<usize>,
    noheader: bool,
    range: Option<Range>,
    config: Arc<Config>,
}

fn guess_width(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    args: Arguments,
) -> Result<PipelineData, ShellError> {
    use super::guess_width::GuessWidth;
    let input_span = input.span().unwrap_or(call.head);

    let mut input = input.collect_string("", &args.config)?;
    if let Some(rows) = args.num_rows_to_skip {
        input = input.lines().skip(rows).map(|x| x.to_string()).join("\n");
    }

    let mut guess_width = GuessWidth::new_reader(Box::new(Cursor::new(input)));

    let result = guess_width.read_all();

    if result.is_empty() {
        return Ok(Value::nothing(input_span).into_pipeline_data());
    }
    if !args.noheader {
        let columns = result[0].clone();
        Ok(result
            .into_iter()
            .skip(1)
            .map(move |s| {
                let mut values: Vec<Value> = s
                    .into_iter()
                    .map(|v| Value::string(v, input_span))
                    .collect();
                // some rows may has less columns, fill it with ""
                for _ in values.len()..columns.len() {
                    values.push(Value::string("", input_span));
                }
                let record =
                    Record::from_raw_cols_vals(columns.clone(), values, input_span, input_span);
                match record {
                    Ok(r) => match &args.range {
                        Some(range) => merge_record(r, range, input_span),
                        None => Value::record(r, input_span),
                    },
                    Err(e) => Value::error(e, input_span),
                }
            })
            .into_pipeline_data(input_span, engine_state.signals().clone()))
    } else {
        let length = result[0].len();
        let columns: Vec<String> = (0..length).map(|n| format!("column{n}")).collect();
        Ok(result
            .into_iter()
            .map(move |s| {
                let mut values: Vec<Value> = s
                    .into_iter()
                    .map(|v| Value::string(v, input_span))
                    .collect();
                // some rows may has less columns, fill it with ""
                for _ in values.len()..columns.len() {
                    values.push(Value::string("", input_span));
                }
                let record =
                    Record::from_raw_cols_vals(columns.clone(), values, input_span, input_span);
                match record {
                    Ok(r) => match &args.range {
                        Some(range) => merge_record(r, range, input_span),
                        None => Value::record(r, input_span),
                    },
                    Err(e) => Value::error(e, input_span),
                }
            })
            .into_pipeline_data(input_span, engine_state.signals().clone()))
    }
}

fn detect_columns(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    args: Arguments,
) -> Result<PipelineData, ShellError> {
    let name_span = call.head;
    let input_span = input.span().unwrap_or(Span::unknown());
    let input = input.collect_string("", &args.config)?;

    let input: Vec<_> = input
        .lines()
        .skip(args.num_rows_to_skip.unwrap_or_default())
        .map(|x| x.to_string())
        .collect();

    let mut input = input.into_iter();
    let headers = input.next();

    if let Some(orig_headers) = headers {
        let mut headers = find_columns(&orig_headers);

        if args.noheader {
            for header in headers.iter_mut().enumerate() {
                header.1.item = format!("column{}", header.0);
            }
        }

        Ok(args
            .noheader
            .then_some(orig_headers)
            .into_iter()
            .chain(input)
            .map(move |x| {
                let row = find_columns(&x);

                let mut record = Record::new();

                if headers.len() == row.len() {
                    for (header, val) in headers.iter().zip(row.iter()) {
                        record.push(&header.item, Value::string(&val.item, name_span));
                    }
                } else {
                    let mut pre_output = vec![];

                    // column counts don't line up, so see if we can figure out why
                    for cell in row {
                        for header in &headers {
                            if cell.span.start <= header.span.end
                                && cell.span.end > header.span.start
                            {
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
                                record.push(&header.item, pre_o.1.clone());
                            }
                        }
                    }
                }

                let has_column_duplicates = record.columns().duplicates().count() > 0;
                if has_column_duplicates {
                    return Err(ShellError::ColumnDetectionFailure {
                        bad_value: input_span,
                        failure_site: name_span,
                    });
                }

                Ok(match &args.range {
                    Some(range) => merge_record(record, range, name_span),
                    None => Value::record(record, name_span),
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_pipeline_data(call.head, engine_state.signals().clone()))
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

fn merge_record(record: Record, range: &Range, input_span: Span) -> Value {
    let (start_index, end_index) = match process_range(range, record.len(), input_span) {
        Ok(Some((l_idx, r_idx))) => (l_idx, r_idx),
        Ok(None) => return Value::record(record, input_span),
        Err(e) => return Value::error(e, input_span),
    };

    match merge_record_impl(record, start_index, end_index, input_span) {
        Ok(rec) => Value::record(rec, input_span),
        Err(err) => Value::error(err, input_span),
    }
}

fn process_range(
    range: &Range,
    length: usize,
    input_span: Span,
) -> Result<Option<(usize, usize)>, ShellError> {
    match nu_cmd_base::util::process_range(range) {
        Ok((l_idx, r_idx)) => {
            let l_idx = if l_idx < 0 {
                length as isize + l_idx
            } else {
                l_idx
            };

            let r_idx = if r_idx < 0 {
                length as isize + r_idx
            } else {
                r_idx
            };

            if !(l_idx <= r_idx && (r_idx >= 0 || l_idx < (length as isize))) {
                return Ok(None);
            }

            Ok(Some((
                l_idx.max(0) as usize,
                (r_idx as usize + 1).min(length),
            )))
        }
        Err(processing_error) => Err(processing_error("could not find range index", input_span)),
    }
}

fn merge_record_impl(
    record: Record,
    start_index: usize,
    end_index: usize,
    input_span: Span,
) -> Result<Record, ShellError> {
    let (mut cols, mut vals): (Vec<_>, Vec<_>) = record.into_iter().unzip();
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
        .map(|v| v.coerce_str().unwrap_or_default())
        .join(" ");
    let binding = Value::string(combined, Span::unknown());
    let last_seg = vals.split_off(end_index);
    vals.truncate(start_index);
    vals.push(binding);
    vals.extend(last_seg);

    Record::from_raw_cols_vals(cols, vals, Span::unknown(), input_span)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(DetectColumns)
    }
}
