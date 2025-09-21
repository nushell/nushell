use std::ops::Bound;

use crate::{grapheme_flags, grapheme_flags_const};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::{IntRange, engine::StateWorkingSet};
use unicode_segmentation::UnicodeSegmentation;

struct Arguments {
    end: bool,
    substring: String,
    range: Option<Spanned<IntRange>>,
    cell_paths: Option<Vec<CellPath>>,
    graphemes: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct StrIndexOf;

impl Command for StrIndexOf {
    fn name(&self) -> &str {
        "str index-of"
    }

    fn signature(&self) -> Signature {
        Signature::build("str index-of")
            .input_output_types(vec![
                (Type::String, Type::Int),
                (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::Int))),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .required("string", SyntaxShape::String, "The string to find in the input.")
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
                "For a data structure input, search strings at the given cell paths, and replace with result.",
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

    fn description(&self) -> &str {
        "Returns start index of first occurrence of string in input, or -1 if no match."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["match", "find", "search"]
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
        let substring: Spanned<String> = call.req(engine_state, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            substring: substring.item,
            range: call.get_flag(engine_state, stack, "range")?,
            end: call.has_flag(engine_state, stack, "end")?,
            cell_paths,
            graphemes: grapheme_flags(engine_state, stack, call)?,
        };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let substring: Spanned<String> = call.req_const(working_set, 0)?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            substring: substring.item,
            range: call.get_flag_const(working_set, "range")?,
            end: call.has_flag_const(working_set, "end")?,
            cell_paths,
            graphemes: grapheme_flags_const(working_set, call)?,
        };
        operate(
            action,
            args,
            input,
            call.head,
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Returns index of string in input",
                example: " 'my_library.rb' | str index-of '.rb'",
                result: Some(Value::test_int(10)),
            },
            Example {
                description: "Count length using grapheme clusters",
                example: "'🇯🇵ほげ ふが ぴよ' | str index-of --grapheme-clusters 'ふが'",
                result: Some(Value::test_int(4)),
            },
            Example {
                description: "Returns index of string in input within a`rhs open range`",
                example: " '.rb.rb' | str index-of '.rb' --range 1..",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: "Returns index of string in input within a lhs open range",
                example: " '123456' | str index-of '6' --range ..4",
                result: Some(Value::test_int(-1)),
            },
            Example {
                description: "Returns index of string in input within a range",
                example: " '123456' | str index-of '3' --range 1..4",
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
        substring,
        range,
        end,
        graphemes,
        ..
    }: &Arguments,
    head: Span,
) -> Value {
    match input {
        Value::String { val: s, .. } => {
            let (search_str, start_index) = if let Some(spanned_range) = range {
                let range_span = spanned_range.span;
                let range = &spanned_range.item;

                let (start, end) = range.absolute_bounds(s.len());
                let s = match end {
                    Bound::Excluded(end) => s.get(start..end),
                    Bound::Included(end) => s.get(start..=end),
                    Bound::Unbounded => s.get(start..),
                };

                let s = match s {
                    Some(s) => s,
                    None => {
                        return Value::error(
                            ShellError::OutOfBounds {
                                left_flank: start.to_string(),
                                right_flank: match range.end() {
                                    Bound::Unbounded => "".to_string(),
                                    Bound::Included(end) => format!("={end}"),
                                    Bound::Excluded(end) => format!("<{end}"),
                                },
                                span: range_span,
                            },
                            head,
                        );
                    }
                };
                (s, start)
            } else {
                (s.as_str(), 0)
            };

            // When the -e flag is present, search using rfind instead of find.s
            if let Some(result) = if *end {
                search_str.rfind(&**substring)
            } else {
                search_str.find(&**substring)
            } {
                let result = result + start_index;
                Value::int(
                    if *graphemes {
                        // Having found the substring's byte index, convert to grapheme index.
                        // grapheme_indices iterates graphemes alongside their UTF-8 byte indices, so .enumerate()
                        // is used to get the grapheme index alongside it.
                        s.grapheme_indices(true)
                            .enumerate()
                            .find(|e| e.1.0 >= result)
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
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.span(),
            },
            head,
        ),
    }
}

#[cfg(test)]
mod tests {
    use nu_protocol::ast::RangeInclusion;

    use super::*;
    use super::{Arguments, StrIndexOf, action};

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrIndexOf {})
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
        let range = IntRange::new(
            Value::int(1, Span::test_data()),
            Value::nothing(Span::test_data()),
            Value::nothing(Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("Cargo"),

            range: Some(spanned_range),
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
        let range = IntRange::new(
            Value::nothing(Span::test_data()),
            Value::nothing(Span::test_data()),
            Value::int(5, Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("Banana"),

            range: Some(spanned_range),
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
        let range = IntRange::new(
            Value::int(2, Span::test_data()),
            Value::nothing(Span::test_data()),
            Value::int(6, Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("123"),

            range: Some(spanned_range),
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
        let range = IntRange::new(
            Value::int(2, Span::test_data()),
            Value::nothing(Span::test_data()),
            Value::int(5, Span::test_data()),
            RangeInclusion::RightExclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("1"),

            range: Some(spanned_range),
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn use_utf8_bytes() {
        let word = Value::string(String::from("🇯🇵ほげ ふが ぴよ"), Span::test_data());

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

    #[test]
    fn index_is_not_a_char_boundary() {
        let word = Value::string(String::from("💛"), Span::test_data());

        let range = IntRange::new(
            Value::int(0, Span::test_data()),
            Value::int(1, Span::test_data()),
            Value::int(2, Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::new(),

            range: Some(spanned_range),
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert!(actual.is_error());
    }

    #[test]
    fn index_is_out_of_bounds() {
        let word = Value::string(String::from("hello"), Span::test_data());

        let range = IntRange::new(
            Value::int(-1, Span::test_data()),
            Value::int(1, Span::test_data()),
            Value::int(3, Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("h"),

            range: Some(spanned_range),
            cell_paths: None,
            end: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }
}
