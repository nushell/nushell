use std::ops::Bound;

use crate::{grapheme_flags, grapheme_flags_const};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::{IntRange, engine::StateWorkingSet};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct StrSubstring;

struct Arguments {
    range: IntRange,
    cell_paths: Option<Vec<CellPath>>,
    graphemes: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

impl Command for StrSubstring {
    fn name(&self) -> &str {
        "str substring"
    }

    fn signature(&self) -> Signature {
        Signature::build("str substring")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::String))),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .switch(
                "grapheme-clusters",
                "count indexes and split using grapheme clusters (all visible chars have length 1)",
                Some('g'),
            )
            .switch(
                "utf-8-bytes",
                "count indexes and split using UTF-8 bytes (default; non-ASCII chars have length 2+)",
                Some('b'),
            )
            .required(
                "range",
                SyntaxShape::Any,
                "The indexes to substring [start end].",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, turn strings at the given cell paths into substrings.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Get part of a string. Note that the first character of a string is index 0."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["slice"]
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
        let range: IntRange = call.req(engine_state, stack, 0)?;

        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            range,
            cell_paths,
            graphemes: grapheme_flags(engine_state, stack, call)?,
        };
        operate(action, args, input, call.head, engine_state.signals()).map(|pipeline| {
            // a substring of text/json is not necessarily text/json itself
            let metadata = pipeline.metadata().map(|m| m.with_content_type(None));
            pipeline.set_metadata(metadata)
        })
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let range: IntRange = call.req_const(working_set, 0)?;

        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            range,
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
        .map(|pipeline| {
            // a substring of text/json is not necessarily text/json itself
            let metadata = pipeline.metadata().map(|m| m.with_content_type(None));
            pipeline.set_metadata(metadata)
        })
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get a substring \"nushell\" from the text \"good nushell\" using a range",
                example: " 'good nushell' | str substring 5..11",
                result: Some(Value::test_string("nushell")),
            },
            Example {
                description: "Count indexes and split using grapheme clusters",
                example: " '🇯🇵ほげ ふが ぴよ' | str substring --grapheme-clusters 4..5",
                result: Some(Value::test_string("ふが")),
            },
            Example {
                description: "sub string by negative index",
                example: " 'good nushell' | str substring 5..-2",
                result: Some(Value::test_string("nushel")),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    match input {
        Value::String { val: s, .. } => {
            let s = if args.graphemes {
                let indices = s
                    .grapheme_indices(true)
                    .map(|(idx, s)| (idx, s.len()))
                    .collect::<Vec<_>>();

                let (idx_start, idx_end) = args.range.absolute_bounds(indices.len());
                let idx_range = match idx_end {
                    Bound::Excluded(end) => &indices[idx_start..end],
                    Bound::Included(end) => &indices[idx_start..=end],
                    Bound::Unbounded => &indices[idx_start..],
                };

                if let Some((start, end)) = idx_range.first().zip(idx_range.last()) {
                    let start = start.0;
                    let end = end.0 + end.1;
                    s[start..end].to_owned()
                } else {
                    String::new()
                }
            } else {
                let (start, end) = args.range.absolute_bounds(s.len());
                let s = match end {
                    Bound::Excluded(end) => &s.as_bytes()[start..end],
                    Bound::Included(end) => &s.as_bytes()[start..=end],
                    Bound::Unbounded => &s.as_bytes()[start..],
                };
                String::from_utf8_lossy(s).into_owned()
            };
            Value::string(s, head)
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::UnsupportedInput {
                msg: "Only string values are supported".into(),
                input: format!("input type: {:?}", other.get_type()),
                msg_span: head,
                input_span: other.span(),
            },
            head,
        ),
    }
}

#[cfg(test)]
#[allow(clippy::reversed_empty_ranges)]
mod tests {
    use nu_protocol::IntRange;

    use super::{Arguments, Span, StrSubstring, Value, action};

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrSubstring {})
    }

    #[derive(Clone, Copy, Debug)]
    struct RangeHelper {
        start: i64,
        end: i64,
        inclusion: nu_protocol::ast::RangeInclusion,
    }

    #[derive(Debug)]
    struct Expectation<'a> {
        range: RangeHelper,
        expected: &'a str,
    }

    impl From<std::ops::RangeInclusive<i64>> for RangeHelper {
        fn from(value: std::ops::RangeInclusive<i64>) -> Self {
            RangeHelper {
                start: *value.start(),
                end: *value.end(),
                inclusion: nu_protocol::ast::RangeInclusion::Inclusive,
            }
        }
    }

    impl From<std::ops::Range<i64>> for RangeHelper {
        fn from(value: std::ops::Range<i64>) -> Self {
            RangeHelper {
                start: value.start,
                end: value.end,
                inclusion: nu_protocol::ast::RangeInclusion::RightExclusive,
            }
        }
    }

    impl From<RangeHelper> for IntRange {
        fn from(value: RangeHelper) -> Self {
            match IntRange::new(
                Value::test_int(value.start),
                Value::test_int(value.start + (if value.start <= value.end { 1 } else { -1 })),
                Value::test_int(value.end),
                value.inclusion,
                Span::test_data(),
            ) {
                Ok(val) => val,
                Err(e) => {
                    panic!("{value:?}: {e:?}")
                }
            }
        }
    }

    impl Expectation<'_> {
        fn range(&self) -> IntRange {
            self.range.into()
        }
    }

    fn expectation(word: &str, range: impl Into<RangeHelper>) -> Expectation {
        Expectation {
            range: range.into(),
            expected: word,
        }
    }

    #[test]
    fn substrings_indexes() {
        let word = Value::test_string("andres");

        let cases = vec![
            expectation("", 0..0),
            expectation("a", 0..=0),
            expectation("an", 0..=1),
            expectation("and", 0..=2),
            expectation("andr", 0..=3),
            expectation("andre", 0..=4),
            expectation("andres", 0..=5),
            expectation("andres", 0..=6),
            expectation("a", 0..=-6),
            expectation("an", 0..=-5),
            expectation("and", 0..=-4),
            expectation("andr", 0..=-3),
            expectation("andre", 0..=-2),
            expectation("andres", 0..=-1),
            // str substring [ -4 , _ ]
            // str substring   -4 ,
            expectation("dres", -4..=i64::MAX),
            expectation("", 0..=-110),
            expectation("", 6..=0),
            expectation("", 6..=-1),
            expectation("", 6..=-2),
            expectation("", 6..=-3),
            expectation("", 6..=-4),
            expectation("", 6..=-5),
            expectation("", 6..=-6),
        ];

        for expectation in &cases {
            println!("{expectation:?}");
            let expected = expectation.expected;
            let actual = action(
                &word,
                &Arguments {
                    range: expectation.range(),
                    cell_paths: None,
                    graphemes: false,
                },
                Span::test_data(),
            );

            assert_eq!(actual, Value::test_string(expected));
        }
    }

    #[test]
    fn use_utf8_bytes() {
        let word = Value::string(String::from("🇯🇵ほげ ふが ぴよ"), Span::test_data());

        let range: RangeHelper = (4..=5).into();
        let options = Arguments {
            cell_paths: None,
            range: range.into(),
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_string("�"));
    }
}
