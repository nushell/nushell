use fancy_regex::Regex;
use nu_engine::command_prelude::*;
use nu_protocol::ListStream;
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
            .input_output_types(vec![(Type::String, Type::table())])
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
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        stats(engine_state, call, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        stats(working_set.permanent(), call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Count the number of words in a string",
                example: r#""There are seven words in this sentence" | str stats"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                            "lines" =>     Value::test_int(1),
                            "words" =>     Value::test_int(7),
                            "bytes" =>     Value::test_int(38),
                            "chars" =>     Value::test_int(38),
                            "graphemes" => Value::test_int(38),
                            "unicode-width" => Value::test_int(38),
                })])),
            },
            Example {
                description: "Counts unicode characters",
                example: r#"'今天天气真好' | str stats"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                            "lines" =>     Value::test_int(1),
                            "words" =>     Value::test_int(6),
                            "bytes" =>     Value::test_int(18),
                            "chars" =>     Value::test_int(6),
                            "graphemes" => Value::test_int(6),
                            "unicode-width" => Value::test_int(12),
                })])),
            },
            Example {
                description: "Counts Unicode characters correctly in a string",
                example: r#""Amélie Amelie" | str stats"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                            "lines" =>     Value::test_int(1),
                            "words" =>     Value::test_int(2),
                            "bytes" =>     Value::test_int(14),
                            "chars" =>     Value::test_int(13),
                            "graphemes" => Value::test_int(13),
                            "unicode-width" => Value::test_int(13),
                })])),
            },
        ]
    }
}

fn stats(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(call.head);
    // This doesn't match explicit nulls
    if matches!(input, PipelineData::Empty) {
        return Err(ShellError::PipelineEmpty { dst_span: span });
    }
    let signals = engine_state.signals().clone();

    // Process each value as it comes in without buffering
    let stream = input.into_iter().map(move |value| {
        let value_span = value.span();
        match value {
            Value::Error { error, .. } => Value::error(*error, value_span),
            value => match value.coerce_str() {
                Ok(s) => counter(s.as_ref(), value_span),
                Err(_) => Value::error(
                    ShellError::PipelineMismatch {
                        exp_input_type: "string".into(),
                        dst_span: span,
                        src_span: value_span,
                    },
                    value_span,
                ),
            },
        }
    });

    Ok(PipelineData::ListStream(
        ListStream::new(stream, span, signals),
        None,
    ))
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
