use crate::{grapheme_flags, grapheme_flags_const};
use nu_cmd_base::{
    input_handler::{operate, CmdArgument},
    util,
};
use nu_engine::command_prelude::*;
use nu_protocol::{engine::StateWorkingSet, Range};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct SubCommand;

struct Arguments {
    indexes: Substring,
    cell_paths: Option<Vec<CellPath>>,
    graphemes: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
struct Substring(isize, isize);

impl From<(isize, isize)> for Substring {
    fn from(input: (isize, isize)) -> Substring {
        Substring(input.0, input.1)
    }
}

impl Command for SubCommand {
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
        let range: Range = call.req(engine_state, stack, 0)?;

        let indexes = match util::process_range(&range) {
            Ok(idxs) => idxs.into(),
            Err(processing_error) => {
                return Err(processing_error("could not perform substring", call.head))
            }
        };

        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            indexes,
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
        let range: Range = call.req_const(working_set, 0)?;

        let indexes = match util::process_range(&range) {
            Ok(idxs) => idxs.into(),
            Err(processing_error) => {
                return Err(processing_error("could not perform substring", call.head))
            }
        };

        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            indexes,
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description:
                    "Get a substring \"nushell\" from the text \"good nushell\" using a range",
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
    let options = &args.indexes;
    match input {
        Value::String { val: s, .. } => {
            let len: isize = s.len() as isize;

            let start: isize = if options.0 < 0 {
                options.0 + len
            } else {
                options.0
            };
            let end: isize = if options.1 < 0 {
                options.1 + len
            } else {
                options.1
            };

            if start > end {
                Value::string("", head)
            } else {
                Value::string(
                    {
                        if end == isize::MAX {
                            if args.graphemes {
                                s.graphemes(true)
                                    .skip(start as usize)
                                    .collect::<Vec<&str>>()
                                    .join("")
                            } else {
                                String::from_utf8_lossy(
                                    &s.bytes().skip(start as usize).collect::<Vec<_>>(),
                                )
                                .to_string()
                            }
                        } else if args.graphemes {
                            s.graphemes(true)
                                .skip(start as usize)
                                .take((end - start + 1) as usize)
                                .collect::<Vec<&str>>()
                                .join("")
                        } else {
                            String::from_utf8_lossy(
                                &s.bytes()
                                    .skip(start as usize)
                                    .take((end - start + 1) as usize)
                                    .collect::<Vec<_>>(),
                            )
                            .to_string()
                        }
                    },
                    head,
                )
            }
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

    use super::{action, Arguments, Span, SubCommand, Value};

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
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
                Value::test_int(value.start as i64),
                Value::test_int(
                    value.start as i64 + (if value.start <= value.end { 1 } else { -1 }),
                ),
                Value::test_int(value.end as i64),
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
            println!("{:?}", expectation);
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
