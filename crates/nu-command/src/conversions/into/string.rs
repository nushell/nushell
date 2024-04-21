use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;
use nu_protocol::{into_code, Config};
use nu_utils::get_system_locale;
use num_format::ToFormattedString;

struct Arguments {
    decimals_value: Option<i64>,
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
                (Type::Glob, Type::String),
                (Type::Bool, Type::String),
                (Type::Filesize, Type::String),
                (Type::Date, Type::String),
                (Type::Duration, Type::String),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
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
        "Convert value to string."
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
    ) -> Result<PipelineData, ShellError> {
        string_helper(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert int to string and append three decimal places",
                example: "5 | into string --decimals 3",
                result: Some(Value::test_string("5.000")),
            },
            Example {
                description: "convert float to string and round to nearest integer",
                example: "1.7 | into string --decimals 0",
                result: Some(Value::test_string("2")),
            },
            Example {
                description: "convert float to string",
                example: "1.7 | into string --decimals 1",
                result: Some(Value::test_string("1.7")),
            },
            Example {
                description: "convert float to string and limit to 2 decimals",
                example: "1.734 | into string --decimals 2",
                result: Some(Value::test_string("1.73")),
            },
            Example {
                description: "convert float to string",
                example: "4.3 | into string",
                result: Some(Value::test_string("4.3")),
            },
            Example {
                description: "convert string to string",
                example: "'1234' | into string",
                result: Some(Value::test_string("1234")),
            },
            Example {
                description: "convert boolean to string",
                example: "true | into string",
                result: Some(Value::test_string("true")),
            },
            Example {
                description: "convert date to string",
                example: "'2020-10-10 10:00:00 +02:00' | into datetime | into string",
                result: Some(Value::test_string("Sat Oct 10 10:00:00 2020")),
            },
            Example {
                description: "convert filepath to string",
                example: "ls Cargo.toml | get name | into string",
                result: None,
            },
            Example {
                description: "convert filesize to string",
                example: "1KiB | into string",
                result: Some(Value::test_string("1.0 KiB")),
            },
            Example {
                description: "convert duration to string",
                example: "9day | into string",
                result: Some(Value::test_string("1wk 2day")),
            },
        ]
    }
}

fn string_helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let decimals_value: Option<i64> = call.get_flag(engine_state, stack, "decimals")?;
    if let Some(decimal_val) = decimals_value {
        if decimal_val.is_negative() {
            return Err(ShellError::TypeMismatch {
                err_message: "Cannot accept negative integers for decimals arguments".to_string(),
                span: head,
            });
        }
    }
    let cell_paths = call.rest(engine_state, stack, 0)?;
    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
    let config = engine_state.get_config().clone();
    let args = Arguments {
        decimals_value,
        cell_paths,
        config,
    };

    match input {
        PipelineData::ExternalStream { stdout: None, .. } => {
            Ok(Value::string(String::new(), head).into_pipeline_data())
        }
        PipelineData::ExternalStream {
            stdout: Some(stream),
            ..
        } => {
            // TODO: in the future, we may want this to stream out, converting each to bytes
            let output = stream.into_string()?;
            Ok(Value::string(output.item, head).into_pipeline_data())
        }
        _ => operate(action, args, input, head, engine_state.ctrlc.clone()),
    }
}

fn action(input: &Value, args: &Arguments, span: Span) -> Value {
    let digits = args.decimals_value;
    let config = &args.config;
    match input {
        Value::Int { val, .. } => {
            let decimal_value = digits.unwrap_or(0) as usize;
            let res = format_int(*val, false, decimal_value);
            Value::string(res, span)
        }
        Value::Float { val, .. } => {
            if let Some(decimal_value) = digits {
                let decimal_value = decimal_value as usize;
                Value::string(format!("{val:.decimal_value$}"), span)
            } else {
                Value::string(val.to_string(), span)
            }
        }
        Value::Bool { val, .. } => Value::string(val.to_string(), span),
        Value::Date { val, .. } => Value::string(val.format("%c").to_string(), span),
        Value::String { val, .. } => Value::string(val.to_string(), span),
        Value::Glob { val, .. } => Value::string(val.to_string(), span),

        Value::Filesize { val: _, .. } => {
            Value::string(input.to_expanded_string(", ", config), span)
        }
        Value::Duration { val: _, .. } => Value::string(input.to_expanded_string("", config), span),

        Value::Error { error, .. } => Value::string(into_code(error).unwrap_or_default(), span),
        Value::Nothing { .. } => Value::string("".to_string(), span),
        Value::Record { .. } => Value::error(
            // Watch out for CantConvert's argument order
            ShellError::CantConvert {
                to_type: "string".into(),
                from_type: "record".into(),
                span,
                help: Some("try using the `to nuon` command".into()),
            },
            span,
        ),
        Value::Binary { .. } => Value::error(
            ShellError::CantConvert {
                to_type: "string".into(),
                from_type: "binary".into(),
                span,
                help: Some("try using the `decode` command".into()),
            },
            span,
        ),
        Value::Custom { val, .. } => {
            // Only custom values that have a base value that can be converted to string are
            // accepted.
            val.to_base_value(input.span())
                .and_then(|base_value| match action(&base_value, args, span) {
                    Value::Error { .. } => Err(ShellError::CantConvert {
                        to_type: String::from("string"),
                        from_type: val.type_name(),
                        span,
                        help: Some("this custom value can't be represented as a string".into()),
                    }),
                    success => Ok(success),
                })
                .unwrap_or_else(|err| Value::error(err, span))
        }
        x => Value::error(
            ShellError::CantConvert {
                to_type: String::from("string"),
                from_type: x.get_type().to_string(),
                span,
                help: None,
            },
            span,
        ),
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
