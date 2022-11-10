use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct SubCommand;

struct Arguments {
    indexes: Substring,
    cell_paths: Option<Vec<CellPath>>,
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

struct SubstringText(String, String);

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str substring"
    }

    fn signature(&self) -> Signature {
        Signature::build("str substring")
            .input_output_types(vec![(Type::String, Type::String)])
            .vectorizes_over_list(true)
            .required(
                "range",
                SyntaxShape::Any,
                "the indexes to substring [start end]",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, turn strings at the given cell paths into substrings",
            )
    }

    fn usage(&self) -> &str {
        "Get part of a string. Note that the start is included but the end is excluded, and that the first character of a string is index 0."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["slice"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let range = call.req(engine_state, stack, 0)?;
        let indexes: Substring = process_arguments(&range, call.head)?.into();
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            indexes,
            cell_paths,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description:
                    "Get a substring \"nushell\" from the text \"good nushell\" using a range",
                example: " 'good nushell' | str substring 5..12",
                result: Some(Value::test_string("nushell")),
            },
            Example {
                description: "Alternately, you can pass in a list",
                example: " 'good nushell' | str substring [5 12]",
                result: Some(Value::test_string("nushell")),
            },
            Example {
                description: "Or a simple comma-separated string",
                example: " 'good nushell' | str substring '5,12'",
                result: Some(Value::test_string("nushell")),
            },
            Example {
                description: "Drop the last `n` characters from the string",
                example: " 'good nushell' | str substring ',-5'",
                result: Some(Value::test_string("good nu")),
            },
            Example {
                description: "Get the remaining characters from a starting index",
                example: " 'good nushell' | str substring '5,'",
                result: Some(Value::test_string("nushell")),
            },
            Example {
                description: "Get the characters from the beginning until ending index",
                example: " 'good nushell' | str substring ',7'",
                result: Some(Value::test_string("good nu")),
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
                std::cmp::max(len + options.1, 0)
            } else {
                options.1
            };

            if start < len && end >= 0 {
                match start.cmp(&end) {
                    Ordering::Equal => Value::String {
                        val: "".to_string(),
                        span: head,
                    },
                    Ordering::Greater => Value::Error {
                        error: ShellError::UnsupportedInput(
                            "End must be greater than or equal to Start".to_string(),
                            head,
                        ),
                    },
                    Ordering::Less => Value::String {
                        val: {
                            if end == isize::max_value() {
                                String::from_utf8_lossy(
                                    &s.bytes().skip(start as usize).collect::<Vec<_>>(),
                                )
                                .to_string()
                            } else {
                                String::from_utf8_lossy(
                                    &s.bytes()
                                        .skip(start as usize)
                                        .take((end - start) as usize)
                                        .collect::<Vec<_>>(),
                                )
                                .to_string()
                            }
                        },
                        span: head,
                    },
                }
            } else {
                Value::String {
                    val: "".to_string(),
                    span: head,
                }
            }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with strings.",
                    other.get_type()
                ),
                head,
            ),
        },
    }
}

fn process_arguments(range: &Value, head: Span) -> Result<(isize, isize), ShellError> {
    let search = match range {
        Value::Range { val, .. } => {
            let start = val.from()?;
            let end = val.to()?;
            Ok(SubstringText(start.to_string(), end.to_string()))
        }
        Value::List { vals, .. } => {
            if vals.len() > 2 {
                Err(ShellError::UnsupportedInput(
                    "More than two indices given".to_string(),
                    head,
                ))
            } else {
                let idx: Vec<String> = vals
                    .iter()
                    .map(|v| {
                        match v {
                            Value::Int { val, .. } => Ok(val.to_string()),
                            Value::String { val, .. } => Ok(val.to_string()),
                            _ => Err(ShellError::UnsupportedInput(
                                "could not perform substring. Expecting a string or int"
                                    .to_string(),
                                head,
                            )),
                        }
                        .unwrap_or_else(|_| String::from(""))
                    })
                    .collect();

                let start = idx
                    .get(0)
                    .ok_or_else(|| {
                        ShellError::UnsupportedInput(
                            "could not perform substring".to_string(),
                            head,
                        )
                    })?
                    .to_string();
                let end = idx
                    .get(1)
                    .ok_or_else(|| {
                        ShellError::UnsupportedInput(
                            "could not perform substring".to_string(),
                            head,
                        )
                    })?
                    .to_string();
                Ok(SubstringText(start, end))
            }
        }
        Value::String { val, .. } => {
            let idx: Vec<&str> = val.split(',').collect();

            let start = idx
                .first()
                .ok_or_else(|| {
                    ShellError::UnsupportedInput("could not perform substring".to_string(), head)
                })?
                .to_string();
            let end = idx
                .get(1)
                .ok_or_else(|| {
                    ShellError::UnsupportedInput("could not perform substring".to_string(), head)
                })?
                .to_string();

            Ok(SubstringText(start, end))
        }
        _ => Err(ShellError::UnsupportedInput(
            "could not perform substring".to_string(),
            head,
        )),
    }?;
    let start = match &search {
        SubstringText(start, _) if start.is_empty() || start == "_" => 0,
        SubstringText(start, _) => start.trim().parse().map_err(|_| {
            ShellError::UnsupportedInput("could not perform substring".to_string(), head)
        })?,
    };

    let end = match &search {
        SubstringText(_, end) if end.is_empty() || end == "_" => isize::max_value(),
        SubstringText(_, end) => end.trim().parse().map_err(|_| {
            ShellError::UnsupportedInput("could not perform substring".to_string(), head)
        })?,
    };

    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::{action, Span, SubCommand, Substring, Value};

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
    struct Expectation<'a> {
        options: (isize, isize),
        expected: &'a str,
    }

    impl Expectation<'_> {
        fn options(&self) -> Substring {
            Substring(self.options.0, self.options.1)
        }
    }

    fn expectation(word: &str, indexes: (isize, isize)) -> Expectation {
        Expectation {
            options: indexes,
            expected: word,
        }
    }

    #[test]
    fn substrings_indexes() {
        let word = Value::String {
            val: "andres".to_string(),
            span: Span::test_data(),
        };

        let cases = vec![
            expectation("a", (0, 1)),
            expectation("an", (0, 2)),
            expectation("and", (0, 3)),
            expectation("andr", (0, 4)),
            expectation("andre", (0, 5)),
            expectation("andres", (0, 6)),
            expectation("", (0, -6)),
            expectation("a", (0, -5)),
            expectation("an", (0, -4)),
            expectation("and", (0, -3)),
            expectation("andr", (0, -2)),
            expectation("andre", (0, -1)),
            // str substring [ -4 , _ ]
            // str substring   -4 ,
            expectation("dres", (-4, isize::max_value())),
            expectation("", (0, -110)),
            expectation("", (6, 0)),
            expectation("", (6, -1)),
            expectation("", (6, -2)),
            expectation("", (6, -3)),
            expectation("", (6, -4)),
            expectation("", (6, -5)),
            expectation("", (6, -6)),
        ];

        for expectation in &cases {
            let expected = expectation.expected;
            let actual = action(
                &word,
                &super::Arguments {
                    indexes: expectation.options(),
                    cell_paths: None,
                },
                Span::test_data(),
            );

            assert_eq!(
                actual,
                Value::String {
                    val: expected.to_string(),
                    span: Span::test_data()
                }
            );
        }
    }
}
