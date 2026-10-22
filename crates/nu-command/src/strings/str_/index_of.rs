use std::ops::Bound;

use crate::{grapheme_flags, grapheme_flags_const};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::{IntRange, engine::StateWorkingSet};
use unicode_segmentation::UnicodeSegmentation;

struct Arguments {
    end: bool,
    all: bool,
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
                (Type::String, Type::Any),
                (Type::List(Box::new(Type::String)), Type::Any),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .required("string", SyntaxShape::String, "The string to find in the input.")
            .switch(
                "grapheme-clusters",
                "Count indexes using grapheme clusters (all visible chars have length 1).",
                Some('g'),
            )
            .switch(
                "utf-8-bytes",
                "Count indexes using UTF-8 bytes (default; non-ASCII chars have length 2+).",
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
                "Optional start and/or end index.",
                Some('r'),
            )
            .switch("end", "Search from the end of the input.", Some('e'))
            .switch("all", "Returns all matched indices.", Some('a'))
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Returns start index of first occurrence of string in input, or -1 if no match."
    }

    fn extra_description(&self) -> &str {
        "Use --all to return a list of every matched index instead of just the first (or last, with --end)."
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
            all: call.has_flag(engine_state, stack, "all")?,
            cell_paths,
            graphemes: grapheme_flags(engine_state, stack, call)?,
        };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let substring: Spanned<String> = call.req_const(working_set, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            substring: substring.item,
            range: call.get_flag_const(working_set, stack, "range")?,
            end: call.has_flag_const(working_set, stack, "end")?,
            all: call.has_flag_const(working_set, stack, "all")?,
            cell_paths,
            graphemes: grapheme_flags_const(working_set, stack, call)?,
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
                description: "Returns index of string in input.",
                example: " 'my_library.rb' | str index-of '.rb'",
                result: Some(Value::test_int(10)),
            },
            Example {
                description: "Count length using grapheme clusters.",
                example: "'🇯🇵ほげ ふが ぴよ' | str index-of --grapheme-clusters 'ふが'",
                result: Some(Value::test_int(4)),
            },
            Example {
                description: "A match that falls inside a grapheme cluster is reported as not found.",
                example: "'🇯🇵' | str index-of --grapheme-clusters '🇵'",
                result: Some(Value::test_int(-1)),
            },
            Example {
                description: "Returns index of string in input within a`rhs open range`.",
                example: " '.rb.rb' | str index-of '.rb' --range 1..",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: "Returns index of string in input within a lhs open range.",
                example: " '123456' | str index-of '6' --range ..4",
                result: Some(Value::test_int(-1)),
            },
            Example {
                description: "Returns index of string in input within a range.",
                example: " '123456' | str index-of '3' --range 1..4",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Returns index of string in input.",
                example: " '/this/is/some/path/file.txt' | str index-of '/' -e",
                result: Some(Value::test_int(18)),
            },
            Example {
                description: "Returns all matched indices.",
                example: " 'a.b.c.d' | str index-of '.' --all",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(3),
                    Value::test_int(5),
                ])),
            },
            Example {
                description: "Returns all matched indices, searching from the end.",
                example: " 'a.b.c.d' | str index-of '.' --all --end",
                result: Some(Value::test_list(vec![
                    Value::test_int(5),
                    Value::test_int(3),
                    Value::test_int(1),
                ])),
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
        all,
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

            // Having found a substring's byte index, convert to grapheme index if needed.
            // grapheme_indices iterates graphemes alongside their UTF-8 byte indices, so .enumerate()
            // is used to get the grapheme index alongside it.
            let to_reported_index = |byte_index: usize| -> i64 {
                if *graphemes {
                    s.grapheme_indices(true)
                        .enumerate()
                        .find(|e| e.1.0 >= byte_index)
                        .map_or(-1, |e| e.0 as i64)
                } else {
                    byte_index as i64
                }
            };

            if *all {
                let indices = search_all_indices(search_str, substring, *end)
                    .map(|result| Value::int(to_reported_index(result + start_index), head));
                Value::list(indices.collect(), head)
            } else if let Some(result) = if *end {
                // When the -e flag is present, search using rfind instead of find.
                search_str.rfind(&**substring)
            } else {
                search_str.find(&**substring)
            } {
                Value::int(to_reported_index(result + start_index), head)
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

/// Returns every non-overlapping start index at which `pattern` matches within `haystack`, as
/// byte offsets into `haystack`. When `from_end` is set, matches are found working backward from
/// the end, and so are yielded in descending order (mirrors `bytes index-of --all --end`).
fn search_all_indices<'a>(
    haystack: &'a str,
    pattern: &'a str,
    from_end: bool,
) -> impl Iterator<Item = usize> + 'a {
    let haystack = haystack.as_bytes();
    let pattern = pattern.as_bytes();
    let pattern_len = pattern.len();

    let mut matches = Vec::new();
    if pattern_len == 0 || pattern_len > haystack.len() {
        return matches.into_iter();
    }

    if from_end {
        let mut right = haystack.len();
        while right >= pattern_len {
            let left = right - pattern_len;
            if &haystack[left..right] == pattern {
                matches.push(left);
                right -= pattern_len;
            } else {
                right -= 1;
            }
        }
    } else {
        let mut left = 0;
        while left + pattern_len <= haystack.len() {
            if &haystack[left..left + pattern_len] == pattern {
                matches.push(left);
                left += pattern_len;
            } else {
                left += 1;
            }
        }
    }

    matches.into_iter()
}

#[cfg(test)]
mod tests {
    use nu_protocol::ast::RangeInclusion;

    use super::*;
    use super::{Arguments, StrIndexOf, action};

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(StrIndexOf)
    }

    #[test]
    fn returns_index_of_substring() {
        let word = Value::test_string("Cargo.tomL");

        let options = Arguments {
            substring: String::from(".tomL"),
            range: None,
            cell_paths: None,
            end: false,
            all: false,
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
            all: false,
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
            all: false,
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
            all: false,
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
            all: false,
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
            all: false,
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
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(15));
    }

    #[test]
    fn returns_all_matching_indices() {
        let word = Value::test_string("a.b.c.d");

        let options = Arguments {
            substring: String::from("."),
            range: None,
            cell_paths: None,
            end: false,
            all: true,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(
            actual,
            Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(3),
                Value::test_int(5),
            ])
        );
    }

    #[test]
    fn returns_all_matching_indices_from_end() {
        let word = Value::test_string("a.b.c.d");

        let options = Arguments {
            substring: String::from("."),
            range: None,
            cell_paths: None,
            end: true,
            all: true,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(
            actual,
            Value::test_list(vec![
                Value::test_int(5),
                Value::test_int(3),
                Value::test_int(1),
            ])
        );
    }

    #[test]
    fn returns_all_matching_indices_with_grapheme_clusters() {
        // Each "🇯🇵" is a single grapheme cluster but 8 UTF-8 bytes, so the byte index of each
        // 'a' (8, then 17) must differ from its grapheme index (1, then 3) -- same conversion
        // the single-match case already does, just applied to every match instead of one.
        let word = Value::test_string("🇯🇵a🇯🇵a🇯🇵");

        let options = Arguments {
            substring: String::from("a"),
            range: None,
            cell_paths: None,
            end: false,
            all: true,
            graphemes: true,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(
            actual,
            Value::test_list(vec![Value::test_int(1), Value::test_int(3)])
        );
    }

    #[test]
    fn returns_no_matches_for_empty_pattern_with_all() {
        let word = Value::test_string("abc");

        let options = Arguments {
            substring: String::new(),
            range: None,
            cell_paths: None,
            end: false,
            all: true,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_list(vec![]));
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
            all: false,
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
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }
}
