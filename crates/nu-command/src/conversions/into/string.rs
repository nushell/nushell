use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    into_code, Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature,
    Span, SyntaxShape, Type, Value,
};
use nu_utils::get_system_locale;
use num_format::ToFormattedString;

struct Arguments {
    decimals_value: Option<i64>,
    decimals: bool,
    cell_paths: Option<Vec<CellPath>>,
    config: Config,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into string"
    }

    fn signature(&self) -> Signature {
        Signature::build("into string")
            .input_output_types(vec![
                (Type::Binary, Type::String),
                (Type::Int, Type::String),
                (Type::Number, Type::String),
                (Type::String, Type::String),
                (Type::Bool, Type::String),
                (Type::Filesize, Type::String),
                (Type::Date, Type::String),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .named(
                "decimals",
                SyntaxShape::Int,
                "decimal digits to which to round",
                Some('d'),
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to string"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "text"]
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
                description: "convert integer to string and append three decimal places",
                example: "5 | into string -d 3",
                result: Some(Value::String {
                    val: "5.000".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert decimal to string and round to nearest integer",
                example: "1.7 | into string -d 0",
                result: Some(Value::String {
                    val: "2".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert decimal to string",
                example: "1.7 | into string -d 1",
                result: Some(Value::String {
                    val: "1.7".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert decimal to string and limit to 2 decimals",
                example: "1.734 | into string -d 2",
                result: Some(Value::String {
                    val: "1.73".to_string(),
                    span: Span::test_data(),
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
                //         Span::test_data(),
                //     ),
                // }),
            },
            Example {
                description: "convert decimal to string",
                example: "4.3 | into string",
                result: Some(Value::String {
                    val: "4.3".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert string to string",
                example: "'1234' | into string",
                result: Some(Value::String {
                    val: "1234".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert boolean to string",
                example: "true | into string",
                result: Some(Value::String {
                    val: "true".to_string(),
                    span: Span::test_data(),
                }),
            },
            // TODO: This should work but does not; see https://github.com/nushell/nushell/issues/7032
            // Example {
            //     description: "convert date to string",
            //     example: "'2020-10-10 10:00:00 +02:00' | into datetime | into string",
            //     result: Some(Value::test_string("Sat Oct 10 10:00:00 2020")),
            // },
            Example {
                description: "convert filepath to string",
                example: "ls Cargo.toml | get name | into string",
                result: None,
            },
            Example {
                description: "convert filesize to string",
                example: "1KiB | into string",
                result: Some(Value::test_string("1,024 B")),
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
    if let Some(decimal_val) = decimals_value {
        if decimals && decimal_val.is_negative() {
            return Err(ShellError::UnsupportedInput(
                "Cannot accept negative integers for decimals arguments".to_string(),
                head,
            ));
        }
    }
    let cell_paths = call.rest(engine_state, stack, 0)?;
    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
    let config = engine_state.get_config().clone();
    let args = Arguments {
        decimals_value,
        decimals,
        cell_paths,
        config,
    };

    match input {
        PipelineData::ExternalStream { stdout: None, .. } => Ok(Value::String {
            val: String::new(),
            span: head,
        }
        .into_pipeline_data()),
        PipelineData::ExternalStream {
            stdout: Some(stream),
            ..
        } => {
            // TODO: in the future, we may want this to stream out, converting each to bytes
            let output = stream.into_string()?;
            Ok(Value::String {
                val: output.item,
                span: head,
            }
            .into_pipeline_data())
        }
        _ => operate(action, args, input, head, engine_state.ctrlc.clone()),
    }
}

fn action(input: &Value, args: &Arguments, span: Span) -> Value {
    let decimals = args.decimals;
    let digits = args.decimals_value;
    let config = &args.config;
    match input {
        Value::Int { val, .. } => {
            let decimal_value = digits.unwrap_or(0) as usize;
            let res = format_int(*val, false, decimal_value);
            Value::String { val: res, span }
        }
        Value::Float { val, .. } => {
            if decimals {
                let decimal_value = digits.unwrap_or(2) as usize;
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
            val: input.into_string(", ", config),
            span,
        },
        Value::Error { error } => Value::String {
            val: {
                match into_code(error) {
                    Some(code) => code,
                    None => "".to_string(),
                }
            },
            span,
        },
        Value::Nothing { .. } => Value::String {
            val: "".to_string(),
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
        Value::Binary { .. } => Value::Error {
            error: ShellError::CantConvert(
                "string".into(),
                "binary".into(),
                span,
                Some("try using the `decode` command".into()),
            ),
        },
        x => Value::Error {
            error: ShellError::CantConvert(
                String::from("string"),
                x.get_type().to_string(),
                span,
                None,
            ),
        },
    }
}

fn format_int(int: i64, group_digits: bool, decimals: usize) -> String {
    let locale = get_system_locale();

    let str = if group_digits {
        int.to_formatted_string(&locale)
    } else {
        int.to_string()
    };

    if decimals > 0 {
        let decimal_point = locale.decimal();

        format!(
            "{}{decimal_point}{dummy:0<decimals$}",
            str,
            decimal_point = decimal_point,
            dummy = "",
            decimals = decimals
        )
    } else {
        str
    }
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
