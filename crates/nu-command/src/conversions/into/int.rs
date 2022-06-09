use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

struct Arguments {
    radix: Option<Value>,
    column_paths: Vec<CellPath>,
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into int"
    }

    fn signature(&self) -> Signature {
        Signature::build("into int")
            .named("radix", SyntaxShape::Number, "radix of integer", Some('r'))
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "column paths to convert to int (for table input)",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to integer"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "number", "natural"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        into_int(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to integer in table",
                example: "echo [[num]; ['-5'] [4] [1.5]] | into int num",
                result: None,
            },
            Example {
                description: "Convert string to integer",
                example: "'2' | into int",
                result: Some(Value::Int(2)),
            },
            Example {
                description: "Convert decimal to integer",
                example: "5.9 | into int",
                result: Some(Value::Int(5)),
            },
            Example {
                description: "Convert decimal string to integer",
                example: "'5.9' | into int",
                result: Some(Value::Int(5)),
            },
            Example {
                description: "Convert file size to integer",
                example: "4KB | into int",
                result: Some(Value::Int(4000)),
            },
            Example {
                description: "Convert bool to integer",
                example: "[false, true] | into int",
                result: Some(Value::List(vec![Value::Int(0), Value::Int(1)])),
            },
            Example {
                description: "Convert date to integer (Unix timestamp)",
                example: "2022-02-02 | into int",
                result: Some(Value::Int(1643760000)),
            },
            Example {
                description: "Convert to integer from binary",
                example: "'1101' | into int -r 2",
                result: Some(Value::Int(13)),
            },
            Example {
                description: "Convert to integer from hex",
                example: "'FF' |  into int -r 16",
                result: Some(Value::Int(255)),
            },
        ]
    }
}

fn into_int(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let head = call.head;

    let options = Arguments {
        radix: call.get_flag(engine_state, stack, "radix")?,
        column_paths: call.rest(engine_state, stack, 0)?,
    };

    let radix: u32 = match options.radix {
        Some(Value::Int(val)) => val as u32,
        Some(_) => 10,
        None => 10,
    };

    if let Some(val) = &options.radix {
        if !(2..=36).contains(&radix) {
            return Err(ShellError::UnsupportedInput(
                "Radix must lie in the range [2, 36]".to_string(),
                val.span()?,
            ));
        }
    }

    input.map(
        move |v| {
            if options.column_paths.is_empty() {
                action(&v, head, radix)
            } else {
                let mut ret = v;
                for path in &options.column_paths {
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, head, radix)),
                    );
                    if let Err(error) = r {
                        return Value::Error(error);
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

pub fn action(input: &Value, span: Span, radix: u32) -> Value {
    match input {
        Value::Int { val: _, .. } => {
            if radix == 10 {
                input.clone()
            } else {
                convert_int(input, span, radix)
            }
        }
        Value::Filesize(val) => Value::Int(*val),
        Value::Float(val) => Value::Int(*val as i64),
        Value::String(val) => {
            if radix == 10 {
                match int_from_string(val, span) {
                    Ok(val) => Value::Int(val),
                    Err(error) => Value::Error(error),
                }
            } else {
                convert_int(input, span, radix)
            }
        }
        Value::Bool(val) => {
            if *val {
                Value::Int(1)
            } else {
                Value::Int(0)
            }
        }
        Value::Date(val) => Value::Int(val.timestamp()),
        _ => Value::Error(ShellError::UnsupportedInput(
            "'into int' for unsupported type".into(),
            span,
        )),
    }
}

fn convert_int(input: &Value, head: Span, radix: u32) -> Value {
    let i = match input {
        Value::Int(val) => val.to_string(),
        Value::String(val) => {
            if val.starts_with("0x") || val.starts_with("0b") {
                match int_from_string(val, head) {
                    Ok(x) => return Value::Int(x),
                    Err(e) => return Value::Error(e),
                }
            }
            val.to_string()
        }
        _ => {
            return Value::Error(ShellError::UnsupportedInput(
                "only strings or integers are supported".to_string(),
                head,
            ))
        }
    };
    match i64::from_str_radix(&i, radix) {
        Ok(n) => Value::Int(n),
        Err(_reason) => Value::Error(ShellError::CantConvert(
            "int".to_string(),
            "string".to_string(),
            head,
            None,
        )),
    }
}

fn int_from_string(a_string: &str, span: Span) -> Result<i64, ShellError> {
    let trimmed = a_string.trim();
    match trimmed {
        b if b.starts_with("0b") => {
            let num = match i64::from_str_radix(b.trim_start_matches("0b"), 2) {
                Ok(n) => n,
                Err(_reason) => {
                    return Err(ShellError::CantConvert(
                        "int".to_string(),
                        "string".to_string(),
                        span,
                        Some(r#"digits following "0b" can only be 0 or 1"#.to_string()),
                    ))
                }
            };
            Ok(num)
        }
        h if h.starts_with("0x") => {
            let num =
                match i64::from_str_radix(h.trim_start_matches("0x"), 16) {
                    Ok(n) => n,
                    Err(_reason) => return Err(ShellError::CantConvert(
                        "int".to_string(),
                        "string".to_string(),
                        span,
                        Some(
                            r#"hexadecimal digits following "0x" should be in 0-9, a-f, or A-F"#
                                .to_string(),
                        ),
                    )),
                };
            Ok(num)
        }
        _ => match a_string.parse::<i64>() {
            Ok(n) => Ok(n),
            Err(_) => match a_string.parse::<f64>() {
                Ok(f) => Ok(f as i64),
                _ => Err(ShellError::CantConvert(
                    "int".to_string(),
                    "string".to_string(),
                    span,
                    None,
                )),
            },
        },
    }
}

#[cfg(test)]
mod test {
    use super::Value;
    use super::*;
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn turns_to_integer() {
        let word = Value::String("10".into());
        let expected = Value::Int(10);

        let actual = action(&word, Span::test_data(), 10);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_binary_to_integer() {
        let s = Value::String("0b101".into());
        let actual = action(&s, Span::test_data(), 10);
        assert_eq!(actual, Value::Int(5));
    }

    #[test]
    fn turns_hex_to_integer() {
        let s = Value::String("0xFF".into());
        let actual = action(&s, Span::test_data(), 16);
        assert_eq!(actual, Value::Int(255));
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_integerlike_string() {
        let integer_str = Value::String("36anra".into());

        let actual = action(&integer_str, Span::test_data(), 10);

        assert_eq!(actual.get_type(), Error)
    }
}
