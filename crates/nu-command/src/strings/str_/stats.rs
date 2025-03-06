use fancy_regex::Regex;
use nu_engine::command_prelude::*;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str stats"
    }

    fn signature(&self) -> Signature {
        Signature::build("str stats")
            .category(Category::Strings)
            .input_output_types(vec![
                (Type::String, Type::table()),
                (Type::list(Type::Any), Type::table()),
            ])
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Gather word count statistics on the text."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["count", "word", "character", "unicode", "wc"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        stats(call, input)
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        stats(call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Count the number of words in a string",
                example: r#""There are seven words in this sentence" | str stats"#,
                result: Some(Value::test_record(record! {
                            "lines" =>     Value::test_int(1),
                            "words" =>     Value::test_int(7),
                            "bytes" =>     Value::test_int(38),
                            "chars" =>     Value::test_int(38),
                            "graphemes" => Value::test_int(38),
                            "unicode-width" => Value::test_int(38),
                })),
            },
            Example {
                description: "Counts unicode characters",
                example: r#"'今天天气真好' | str stats"#,
                result: Some(Value::test_record(record! {
                            "lines" =>     Value::test_int(1),
                            "words" =>     Value::test_int(6),
                            "bytes" =>     Value::test_int(18),
                            "chars" =>     Value::test_int(6),
                            "graphemes" => Value::test_int(6),
                            "unicode-width" => Value::test_int(12),
                })),
            },
            Example {
                description: "Counts Unicode characters correctly in a string",
                example: r#""Amélie Amelie" | str stats"#,
                result: Some(Value::test_record(record! {
                            "lines" =>     Value::test_int(1),
                            "words" =>     Value::test_int(2),
                            "bytes" =>     Value::test_int(14),
                            "chars" =>     Value::test_int(13),
                            "graphemes" => Value::test_int(13),
                            "unicode-width" => Value::test_int(13),
                })),
            },
        ]
    }
}

fn stats(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(call.head);
    let metadata = input.metadata();

    // Initialize counters
    let mut lines = 0;
    let mut words = 0;
    let mut bytes = 0;
    let mut chars = 0;
    let mut graphemes = 0;
    let mut unicode_width = 0;

    match input {
        PipelineData::Empty => Ok(PipelineData::Empty),
        PipelineData::Value(Value::List { .. }, ..) | PipelineData::ListStream(..) => {
            for value in input.into_iter() {
                match process_and_update_stats(value, span, "list/liststream") {
                    Ok((l, w, b, c, g, u)) => {
                        lines += l;
                        words += w;
                        bytes += b;
                        chars += c;
                        graphemes += g;
                        unicode_width += u;
                    }
                    Err(err) => return Ok(Value::error(err, span).into_pipeline_data()),
                }
            }

            Ok(
                create_result(lines, words, bytes, chars, graphemes, unicode_width, span)
                    .into_pipeline_data(),
            )
        }
        PipelineData::ByteStream(stream, ..) => {
            let span = stream.span();
            if let Some(chunks) = stream.chunks() {
                for value in chunks {
                    let val = match value {
                        Ok(v) => v,
                        Err(e) => return Ok(Value::error(e, span).into_pipeline_data()),
                    };

                    match process_and_update_stats(val, span, "bytestream") {
                        Ok((l, w, b, c, g, u)) => {
                            lines += l;
                            words += w;
                            bytes += b;
                            chars += c;
                            graphemes += g;
                            unicode_width += u;
                        }
                        Err(err) => return Ok(Value::error(err, span).into_pipeline_data()),
                    }
                }

                Ok(
                    create_result(lines, words, bytes, chars, graphemes, unicode_width, span)
                        .into_pipeline_data(),
                )
            } else {
                Ok(PipelineData::Empty)
            }
        }
        PipelineData::Value(..) => {
            let metadata_clone = metadata.clone();
            for value in input.into_iter() {
                match process_and_update_stats(value, span, "string") {
                    Ok((l, w, b, c, g, u)) => {
                        lines += l;
                        words += w;
                        bytes += b;
                        chars += c;
                        graphemes += g;
                        unicode_width += u;
                    }
                    Err(err) => return Ok(Value::error(err, span).into_pipeline_data()),
                }
            }

            Ok(
                create_result(lines, words, bytes, chars, graphemes, unicode_width, span)
                    .into_pipeline_data_with_metadata(metadata_clone),
            )
        }
    }
}

fn process_and_update_stats(
    value: Value,
    span: Span,
    input_type: &str,
) -> Result<(i64, i64, i64, i64, i64, i64), ShellError> {
    let value_span = value.span();

    match value {
        Value::Error { error, .. } => Err(*error),
        value => match value.coerce_str() {
            Ok(s) => {
                // Count directly and update stats
                let result = counter(s.as_ref(), value_span);
                if let Value::Record { val, .. } = &result {
                    let lines = val.get("lines").and_then(|v| v.as_int().ok()).unwrap_or(0);
                    let words = val.get("words").and_then(|v| v.as_int().ok()).unwrap_or(0);
                    let bytes = val.get("bytes").and_then(|v| v.as_int().ok()).unwrap_or(0);
                    let chars = val.get("chars").and_then(|v| v.as_int().ok()).unwrap_or(0);
                    let graphemes = val
                        .get("graphemes")
                        .and_then(|v| v.as_int().ok())
                        .unwrap_or(0);
                    let unicode_width = val
                        .get("unicode-width")
                        .and_then(|v| v.as_int().ok())
                        .unwrap_or(0);

                    Ok((lines, words, bytes, chars, graphemes, unicode_width))
                } else {
                    Ok((0, 0, 0, 0, 0, 0))
                }
            }
            Err(_) => Err(ShellError::PipelineMismatch {
                exp_input_type: input_type.into(),
                dst_span: span,
                src_span: value_span,
            }),
        },
    }
}

fn create_result(
    lines: i64,
    words: i64,
    bytes: i64,
    chars: i64,
    graphemes: i64,
    unicode_width: i64,
    span: Span,
) -> Value {
    Value::record(
        record! {
            "lines" => Value::int(lines, span),
            "words" => Value::int(words, span),
            "bytes" => Value::int(bytes, span),
            "chars" => Value::int(chars, span),
            "graphemes" => Value::int(graphemes, span),
            "unicode-width" => Value::int(unicode_width, span),
        },
        span,
    )
}

fn counter(contents: &str, span: Span) -> Value {
    // Do a single pass over the string to count everything
    let mut graphemes = 0;
    let mut codepoints = 0;
    let mut unicode_width = 0;

    // Line ending patterns
    let line_ending_types = [
        "\r\n", "\n", "\r", "\u{0085}", "\u{000C}", "\u{2028}", "\u{2029}",
    ];
    let pattern = line_ending_types.join("|");
    // This unwrap is safe because we're using a hardcoded, valid regex pattern
    let line_regex = Regex::new(&pattern).expect("Invalid regex pattern");

    // Count lines
    let line_endings = line_regex
        .find_iter(contents)
        .filter_map(Result::ok)
        .count();

    let lines = if contents.is_empty() {
        0
    } else if line_ending_types
        .iter()
        .any(|&suffix| contents.ends_with(suffix))
    {
        line_endings
    } else {
        line_endings + 1
    };

    // Count other metrics in a single pass
    for grapheme in contents.graphemes(true) {
        graphemes += 1;
        codepoints += grapheme.chars().count();
        unicode_width += unicode_width::UnicodeWidthStr::width(grapheme);
    }

    let words = contents.unicode_words().count();

    Value::record(
        record! {
            "lines" => Value::int(lines as i64, span),
            "words" => Value::int(words as i64, span),
            "bytes" => Value::int(contents.len() as i64, span),
            "chars" => Value::int(codepoints as i64, span),
            "graphemes" => Value::int(graphemes as i64, span),
            "unicode-width" => Value::int(unicode_width as i64, span),
        },
        span,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;
        test_examples(SubCommand {})
    }
}

#[test]
fn test_one_newline() {
    let s = "\n";
    let result = counter(s, Span::test_data());

    let expected = Value::test_record(record! {
        "lines" => Value::test_int(1),
        "words" => Value::test_int(0),
        "bytes" => Value::test_int(1),
        "chars" => Value::test_int(1),
        "graphemes" => Value::test_int(1),
        "unicode-width" => Value::test_int(1),
    });

    assert_eq!(result, expected);
}

#[test]
fn test_count_counts_lines() {
    // const LF: &str = "\n"; // 0xe0000a
    // const CR: &str = "\r"; // 0xe0000d
    // const CRLF: &str = "\r\n"; // 0xe00d0a
    const NEL: &str = "\u{0085}"; // 0x00c285
    const FF: &str = "\u{000C}"; // 0x00000c
    const LS: &str = "\u{2028}"; // 0xe280a8
    const PS: &str = "\u{2029}"; // 0xe280a9

    // * \r\n is a single grapheme cluster
    // * trailing newlines are counted
    // * NEL is 2 bytes
    // * FF is 1 byte
    // * LS is 3 bytes
    // * PS is 3 bytes
    let mut s = String::from("foo\r\nbar\n\nbaz");
    s += NEL;
    s += "quux";
    s += FF;
    s += LS;
    s += "xi";
    s += PS;
    s += "\n";

    let result = counter(&s, Span::test_data());

    let expected = Value::test_record(record! {
        "lines" => Value::test_int(8),
        "words" => Value::test_int(5),
        "bytes" => Value::test_int(29),
        "chars" => Value::test_int(24),
        "graphemes" => Value::test_int(23),
        "unicode-width" => Value::test_int(23),
    });

    assert_eq!(result, expected);
}

#[test]
fn test_count_counts_words() {
    let i_can_eat_glass = "Μπορῶ νὰ φάω σπασμένα γυαλιὰ χωρὶς νὰ πάθω τίποτα.";

    let result = counter(i_can_eat_glass, Span::test_data());

    let expected = Value::test_record(record! {
        "lines" => Value::test_int(1),
        "words" => Value::test_int(9),
        "bytes" => Value::test_int(i_can_eat_glass.len() as i64),
        "chars" => Value::test_int(50),
        "graphemes" => Value::test_int(50),
        "unicode-width" => Value::test_int(50),
    });

    assert_eq!(result, expected);
}

#[test]
fn test_count_counts_codepoints() {
    // these are NOT the same! One is e + ́́ , and one is é, a single codepoint
    let one = "é"; // single codepoint
    let two = "e\u{0301}"; // e + combining acute accent

    let result_one = counter(one, Span::test_data());
    let result_two = counter(two, Span::test_data());

    assert_eq!(
        result_one
            .as_record()
            .expect("record exists")
            .get("chars")
            .expect("chars field exists")
            .as_int()
            .expect("is int"),
        1
    );
    assert_eq!(
        result_two
            .as_record()
            .expect("record exists")
            .get("chars")
            .expect("chars field exists")
            .as_int()
            .expect("is int"),
        2
    );
}
