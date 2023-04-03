use crate::input_handler::{operate, CmdArgument};
use crate::{grapheme_flags, util};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, Range, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
};
use unicode_segmentation::UnicodeSegmentation;

struct Arguments {
    end: bool,
    substring: String,
    range: Option<Range>,
    cell_paths: Option<Vec<CellPath>>,
    graphemes: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct SubCommand;

#[derive(Clone)]
pub struct IndexOfOptionalBounds(i32, i32);

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str index-of"
    }

    fn signature(&self) -> Signature {
        Signature::build("str index-of")
            .input_output_types(vec![(Type::String, Type::Int)])
            .vectorizes_over_list(true) // TODO: no test coverage
            .required("string", SyntaxShape::String, "the string to find in the input")
            .switch(
                "grapheme-clusters",
                "count indexes using grapheme clusters (all visible chars have length 1)",
                Some('g'),
            )
            .switch(
                "utf-8-bytes",
                "count indexes using UTF-8 bytes (default; non-ASCII chars have length 2+)",
                Some('b'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, search strings at the given cell paths, and replace with result",
            )
            .named(
                "range",
                SyntaxShape::Range,
                "optional start and/or end index",
                Some('r'),
            )
            .switch("end", "search from the end of the input", Some('e'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Returns start index of first occurrence of string in input, or -1 if no match."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["match", "find", "search"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let substring: Spanned<String> = call.req(engine_state, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            substring: substring.item,
            range: call.get_flag(engine_state, stack, "range")?,
            end: call.has_flag("end"),
            cell_paths,
            graphemes: grapheme_flags(call)?,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns index of string in input",
                example: " 'my_library.rb' | str index-of '.rb'",
                result: Some(Value::test_int(10)),
            },
            Example {
                description: "Count length using grapheme clusters",
                example: "'🇯🇵ほげ ふが ぴよ' | str index-of -g 'ふが'",
                result: Some(Value::test_int(4)),
            },
            Example {
                description: "Returns index of string in input within a`rhs open range`",
                example: " '.rb.rb' | str index-of '.rb' -r 1..",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: "Returns index of string in input within a lhs open range",
                example: " '123456' | str index-of '6' -r ..4",
                result: Some(Value::test_int(-1)),
            },
            Example {
                description: "Returns index of string in input within a range",
                example: " '123456' | str index-of '3' -r 1..4",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Returns index of string in input",
                example: " '/this/is/some/path/file.txt' | str index-of '/' -e",
                result: Some(Value::test_int(18)),
            },
        ]
    }
}

fn action(
    input: &Value,
    Arguments {
        ref substring,
        range,
        end,
        graphemes,
        ..
    }: &Arguments,
    head: Span,
) -> Value {
    match input {
        Value::String { val: s, .. } => {
            let (start_index, end_index) = if let Some(range) = range {
                match util::process_range(range) {
                    Ok(r) => {
                        // `process_range()` returns `isize::MAX` if the range is open-ended,
                        // which is not ideal for us
                        let end = if r.1 as usize > s.len() {
                            s.len()
                        } else {
                            r.1 as usize
                        };
                        (r.0 as usize, end)
                    }
                    Err(processing_error) => {
                        let err = processing_error("could not find `index-of`", head);
                        return Value::Error {
                            error: Box::new(err),
                        };
                    }
                }
            } else {
                (0usize, s.len())
            };

            // When the -e flag is present, search using rfind instead of find.s
            if let Some(result) = if *end {
                s[start_index..end_index].rfind(&**substring)
            } else {
                s[start_index..end_index].find(&**substring)
            } {
                let result = result + start_index;
                Value::int(
                    if *graphemes {
                        // Having found the substring's byte index, convert to grapheme index.
                        // grapheme_indices iterates graphemes alongside their UTF-8 byte indices, so .enumerate()
                        // is used to get the grapheme index alongside it.
                        s.grapheme_indices(true)
                            .enumerate()
                            .find(|e| e.1 .0 >= result)
                            .expect("No grapheme index for substring")
                            .0
                    } else {
                        result
                    } as i64,
                    head,
                )
            } else {
                Value::int(-1, head)
            }
        }
        Value::Error { .. } => input.clone(),
        _ => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.expect_span(),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use nu_protocol::ast::RangeInclusion;

    use super::*;
    use super::{action, Arguments, SubCommand};

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn returns_index_of_substring() {
        let word = Value::test_string("Cargo.tomL");

        let options = Arguments {
            substring: String::from(".tomL"),
            range: None,
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());

        assert_eq!(actual, Value::test_int(5));
    }
    #[test]
    fn index_of_does_not_exist_in_string() {
        let word = Value::test_string("Cargo.tomL");

        let options = Arguments {
            substring: String::from("Lm"),

            range: None,
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());

        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn returns_index_of_next_substring() {
        let word = Value::test_string("Cargo.Cargo");
        let range = Range {
            from: Value::Int {
                val: 1,
                span: Span::test_data(),
            },
            incr: Value::Int {
                val: 1,
                span: Span::test_data(),
            },
            to: Value::Nothing {
                span: Span::test_data(),
            },
            inclusion: RangeInclusion::Inclusive,
        };

        let options = Arguments {
            substring: String::from("Cargo"),

            range: Some(range),
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(6));
    }

    #[test]
    fn index_does_not_exist_due_to_end_index() {
        let word = Value::test_string("Cargo.Banana");
        let range = Range {
            from: Value::Nothing {
                span: Span::test_data(),
            },
            inclusion: RangeInclusion::Inclusive,
            incr: Value::Int {
                val: 1,
                span: Span::test_data(),
            },
            to: Value::Int {
                val: 5,
                span: Span::test_data(),
            },
        };

        let options = Arguments {
            substring: String::from("Banana"),

            range: Some(range),
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn returns_index_of_nums_in_middle_due_to_index_limit_from_both_ends() {
        let word = Value::test_string("123123123");
        let range = Range {
            from: Value::Int {
                val: 2,
                span: Span::test_data(),
            },
            incr: Value::Int {
                val: 1,
                span: Span::test_data(),
            },
            to: Value::Int {
                val: 6,
                span: Span::test_data(),
            },
            inclusion: RangeInclusion::Inclusive,
        };

        let options = Arguments {
            substring: String::from("123"),

            range: Some(range),
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(3));
    }

    #[test]
    fn index_does_not_exists_due_to_strict_bounds() {
        let word = Value::test_string("123456");
        let range = Range {
            from: Value::Int {
                val: 2,
                span: Span::test_data(),
            },
            incr: Value::Int {
                val: 1,
                span: Span::test_data(),
            },
            to: Value::Int {
                val: 5,
                span: Span::test_data(),
            },
            inclusion: RangeInclusion::RightExclusive,
        };

        let options = Arguments {
            substring: String::from("1"),

            range: Some(range),
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn use_utf8_bytes() {
        let word = Value::String {
            val: String::from("🇯🇵ほげ ふが ぴよ"),
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("ふが"),

            range: None,
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(15));
    }
}
