use super::state::stream_grep;
use fff_search::GrepMode;
use nu_engine::command_prelude::*;
use nu_protocol::Range;
use std::ops::Bound;

#[derive(Clone)]
pub struct IdxSearch;

impl Command for IdxSearch {
    fn name(&self) -> &str {
        "idx search"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "pattern",
                SyntaxShape::String,
                "One or more search patterns.",
            )
            .switch("regex", "Use regular-expression matching mode.", Some('r'))
            .switch("fuzzy", "Use fuzzy line-matching mode.", Some('f'))
            .named(
                "limit",
                SyntaxShape::Int,
                "Maximum number of matches to collect.",
                Some('l'),
            )
            .named(
                "context",
                SyntaxShape::OneOf(vec![SyntaxShape::Range, SyntaxShape::Int]),
                "The number of context lines to include before and after each match can be specified as an integer or a range. An integer sets both the before and after context to that number, while a range uses a negative value for lines before and a positive value for lines after (e.g., -3..5).",
                Some('c'),
            )
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::record())))])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Search indexed file contents."
    }

    fn extra_description(&self) -> &str {
        "Mode selection: plain text is the default and treats each pattern literally, `--regex` evaluates the patterns as regular expressions, and `--fuzzy` performs approximate line matching."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Search indexed file contents for a plain text pattern.",
                example: "idx search hello",
                result: None,
            },
            Example {
                description: "Search using a regular expression.",
                example: "idx search --regex 'fn \\w+'",
                result: None,
            },
            Example {
                description: "Search with multiple patterns simultaneously.",
                example: "idx search TODO FIXME HACK",
                result: None,
            },
            Example {
                description: "Include 2 lines of context before and 5 lines after each match.",
                example: "idx search error -2..5",
                result: None,
            },
            Example {
                description: "Brackets and question marks are treated as literal text, not glob patterns.",
                example: "idx search 'arr[0]'",
                result: None,
            },
            Example {
                description: "Glob patterns with a path separator filter which files to search.",
                example: "idx search pattern tests/*",
                result: None,
            },
            Example {
                description: "Brace expansion globs also filter which files to search.",
                example: "idx search pattern *.{rs,js}",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let patterns: Vec<String> = call.rest(engine_state, stack, 0)?;
        if patterns.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "pattern".to_string(),
                span: call.head,
            });
        }

        let regex = call.has_flag(engine_state, stack, "regex")?;
        let fuzzy = call.has_flag(engine_state, stack, "fuzzy")?;

        if regex && fuzzy {
            return Err(ShellError::IncompatibleParameters {
                left_message: "--regex cannot be used with --fuzzy".to_string(),
                left_span: call.get_flag_span(stack, "regex").unwrap_or(call.head),
                right_message: "--fuzzy cannot be used with --regex".to_string(),
                right_span: call.get_flag_span(stack, "fuzzy").unwrap_or(call.head),
            });
        }

        let limit = call
            .get_flag::<i64>(engine_state, stack, "limit")?
            .and_then(|v| usize::try_from(v).ok())
            .unwrap_or(50);

        let mode = if fuzzy {
            GrepMode::Fuzzy
        } else if regex {
            GrepMode::Regex
        } else {
            GrepMode::PlainText
        };

        let context_param: Option<Value> = call.get_flag(engine_state, stack, "context")?;
        let (before_context, after_context) = match context_param {
            Some(Value::Int { val: i, .. }) if i < 0 => {
                return Err(ShellError::UnsupportedInput {
                    msg: "Context must be specified as an or a range (e.g. -3..5). Negative value for before-context, positive value for after-context.".into(),
                    input: "value originates from here".into(),
                    msg_span: call.head,
                    input_span: call.head,
                });
            }
            Some(Value::Int { val: n, .. }) => (n as usize, n as usize),
            Some(Value::Range { val: range, .. }) => {
                // Valid cases
                match *range {
                    Range::IntRange(r) => {
                        // Reject three-part ranges like -3..1..5 (explicit step != 1)
                        if r.step() != 1 {
                            return Err(ShellError::UnsupportedInput {
                                msg: "Context range must not have an explicit step (e.g. use -3..5, not -3..1..5)".into(),
                                input: "value originates from here".into(),
                                msg_span: call.head,
                                input_span: call.head,
                            });
                        }

                        let start = r.start();
                        if start > 0 {
                            return Err(ShellError::UnsupportedInput {
                                msg: "Context range start must be <= 0 (use a negative value for before-context, e.g. -3..5)".into(),
                                input: "value originates from here".into(),
                                msg_span: call.head,
                                input_span: call.head,
                            });
                        }

                        let end_val = match r.end() {
                            Bound::Included(e) | Bound::Excluded(e) => e,
                            Bound::Unbounded => {
                                return Err(ShellError::UnsupportedInput {
                                    msg: "Context range must have a bounded end (use a positive value for after-context, e.g. -3..5)".into(),
                                    input: "value originates from here".into(),
                                    msg_span: call.head,
                                    input_span: call.head,
                                });
                            }
                        };

                        if end_val < 0 {
                            return Err(ShellError::UnsupportedInput {
                                msg: "Context range end must be >= 0 (use a positive value for after-context, e.g. -3..5)".into(),
                                input: "value originates from here".into(),
                                msg_span: call.head,
                                input_span: call.head,
                            });
                        }

                        let before = start.unsigned_abs() as usize;
                        let after = end_val as usize;
                        (before, after)
                    }
                    Range::FloatRange(_) => {
                        return Err(ShellError::UnsupportedInput {
                            msg: "Float ranges are not supported for context".into(),
                            input: "value originates from here".into(),
                            msg_span: call.head,
                            input_span: call.head,
                        });
                    }
                }
            }
            Some(other) => {
                return Err(ShellError::UnsupportedInput {
                    msg: format!(
                        "Context must be an integer or range, but got {}",
                        other.get_type()
                    ),
                    input: "value originates from here".into(),
                    msg_span: call.head,
                    input_span: call.head,
                });
            }
            None => (0usize, 0usize),
        };

        let signals = engine_state.signals();
        stream_grep(
            &patterns,
            mode,
            limit,
            call.head,
            before_context,
            after_context,
            signals,
        )
    }
}
