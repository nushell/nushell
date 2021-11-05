use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

// TODO num_format::SystemLocale once platform-specific dependencies are stable (see Cargo.toml)

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into string"
    }

    fn signature(&self) -> Signature {
        Signature::build("into string")
            // FIXME - need to support column paths
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "column paths to convert to string (for table input)",
            )
            .named(
                "decimals",
                SyntaxShape::Int,
                "decimal digits to which to round",
                Some('d'),
            )
    }

    fn usage(&self) -> &str {
        "Convert value to string"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        string_helper(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert decimal to string and round to nearest integer",
                example: "1.7 | into string -d 0",
                result: Some(Value::String {
                    val: "2".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert decimal to string",
                example: "1.7 | into string -d 1",
                result: Some(Value::String {
                    val: "1.7".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert decimal to string and limit to 2 decimals",
                example: "1.734 | into string -d 2",
                result: Some(Value::String {
                    val: "1.73".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "try to convert decimal to string and provide negative decimal points",
                example: "1.734 | into string -d -2",
                result: None,
                // FIXME
                // result: Some(Value::Error {
                //     error: ShellError::UnsupportedInput(
                //         String::from("Cannot accept negative integers for decimals arguments"),
                //         Span::unknown(),
                //     ),
                // }),
            },
            Example {
                description: "convert decimal to string",
                example: "4.3 | into string",
                result: Some(Value::String {
                    val: "4.3".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert string to string",
                example: "'1234' | into string",
                result: Some(Value::String {
                    val: "1234".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert boolean to string",
                example: "$true | into string",
                result: Some(Value::String {
                    val: "true".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert date to string",
                example: "date now | into string",
                result: None,
            },
            Example {
                description: "convert filepath to string",
                example: "ls Cargo.toml | get name | into string",
                result: None,
            },
            Example {
                description: "convert filesize to string",
                example: "ls Cargo.toml | get size | into string",
                result: None,
            },
        ]
    }
}

fn string_helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, ShellError> {
    let decimals = call.has_flag("decimals");
    let head = call.head;
    let decimals_value: Option<i64> = call.get_flag(engine_state, stack, "decimals")?;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    if decimals && decimals_value.is_some() && decimals_value.unwrap().is_negative() {
        return Err(ShellError::UnsupportedInput(
            "Cannot accept negative integers for decimals arguments".to_string(),
            head,
        ));
    }

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, head, decimals, decimals_value, false)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, head, decimals, decimals_value, false)),
                    );
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

pub fn action(
    input: &Value,
    span: Span,
    decimals: bool,
    digits: Option<i64>,
    group_digits: bool,
) -> Value {
    match input {
        Value::Int { val, .. } => {
            let res = if group_digits {
                format_int(*val) // int.to_formatted_string(*locale)
            } else {
                val.to_string()
            };

            Value::String { val: res, span }
        }
        Value::Float { val, .. } => {
            if decimals {
                let decimal_value = digits.unwrap() as usize;
                Value::String {
                    val: format!("{:.*}", decimal_value, val),
                    span,
                }
            } else {
                Value::String {
                    val: val.to_string(),
                    span,
                }
            }
        }
        Value::Bool { val, .. } => Value::String {
            val: val.to_string(),
            span,
        },
        Value::Date { val, .. } => Value::String {
            val: val.format("%c").to_string(),
            span,
        },
        Value::String { val, .. } => Value::String {
            val: val.to_string(),
            span,
        },

        Value::Filesize { val: _, .. } => Value::String {
            val: input.clone().into_string(),
            span,
        },
        Value::Nothing { .. } => Value::String {
            val: "nothing".to_string(),
            span,
        },
        Value::Record {
            cols: _,
            vals: _,
            span: _,
        } => Value::Error {
            error: ShellError::UnsupportedInput(
                "Cannot convert Record into string".to_string(),
                span,
            ),
        },
        _ => Value::Error {
            error: ShellError::CantConvert(
                String::from(" into string. Probably this type is not supported yet"),
                span,
            ),
        },
    }
}
fn format_int(int: i64) -> String {
    int.to_string()

    // TODO once platform-specific dependencies are stable (see Cargo.toml)
    // #[cfg(windows)]
    // {
    //     int.to_formatted_string(&Locale::en)
    // }
    // #[cfg(not(windows))]
    // {
    //     match SystemLocale::default() {
    //         Ok(locale) => int.to_formatted_string(&locale),
    //         Err(_) => int.to_formatted_string(&Locale::en),
    //     }
    // }
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
