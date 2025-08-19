use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
    compact: bool,
    little_endian: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct IntoBinary;

impl Command for IntoBinary {
    fn name(&self) -> &str {
        "into binary"
    }

    fn signature(&self) -> Signature {
        Signature::build("into binary")
            .input_output_types(vec![
                (Type::Binary, Type::Binary),
                (Type::Int, Type::Binary),
                (Type::Number, Type::Binary),
                (Type::String, Type::Binary),
                (Type::Bool, Type::Binary),
                (Type::Filesize, Type::Binary),
                (Type::Date, Type::Binary),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true) // TODO: supply exhaustive examples
            .switch("compact", "output without padding zeros", Some('c'))
            .named(
                "endian",
                SyntaxShape::String,
                "byte encode endian. Does not affect string, date or binary. In containers, only individual elements are affected. Available options: native(default), little, big",
                Some('e'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert value to a binary primitive."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "bytes"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_binary(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert string to a nushell binary primitive",
                example: "'This is a string that is exactly 52 characters long.' | into binary",
                result: Some(Value::binary(
                    "This is a string that is exactly 52 characters long."
                        .to_string()
                        .as_bytes()
                        .to_vec(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert a number to a nushell binary primitive",
                example: "1 | into binary",
                result: Some(Value::binary(
                    i64::from(1).to_ne_bytes().to_vec(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert a number to a nushell binary primitive (big endian)",
                example: "258 | into binary --endian big",
                result: Some(Value::binary(
                    i64::from(258).to_be_bytes().to_vec(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert a number to a nushell binary primitive (little endian)",
                example: "258 | into binary --endian little",
                result: Some(Value::binary(
                    i64::from(258).to_le_bytes().to_vec(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert a boolean to a nushell binary primitive",
                example: "true | into binary",
                result: Some(Value::binary(
                    i64::from(1).to_ne_bytes().to_vec(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert a filesize to a nushell binary primitive",
                example: "ls | where name == LICENSE | get size | into binary",
                result: None,
            },
            Example {
                description: "convert a filepath to a nushell binary primitive",
                example: "ls | where name == LICENSE | get name | path expand | into binary",
                result: None,
            },
            Example {
                description: "convert a float to a nushell binary primitive",
                example: "1.234 | into binary",
                result: Some(Value::binary(
                    1.234f64.to_ne_bytes().to_vec(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert an int to a nushell binary primitive with compact enabled",
                example: "10 | into binary --compact",
                result: Some(Value::binary(vec![10], Span::test_data())),
            },
        ]
    }
}

fn into_binary(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let cell_paths = call.rest(engine_state, stack, 0)?;
    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

    if let PipelineData::ByteStream(stream, metadata) = input {
        // Just set the type - that should be good enough
        Ok(PipelineData::byte_stream(
            stream.with_type(ByteStreamType::Binary),
            metadata,
        ))
    } else {
        let endian = call.get_flag::<Spanned<String>>(engine_state, stack, "endian")?;

        let little_endian = if let Some(endian) = endian {
            match endian.item.as_str() {
                "native" => cfg!(target_endian = "little"),
                "little" => true,
                "big" => false,
                _ => {
                    return Err(ShellError::TypeMismatch {
                        err_message: "Endian must be one of native, little, big".to_string(),
                        span: endian.span,
                    });
                }
            }
        } else {
            cfg!(target_endian = "little")
        };

        let args = Arguments {
            cell_paths,
            compact: call.has_flag(engine_state, stack, "compact")?,
            little_endian,
        };
        operate(action, args, input, head, engine_state.signals())
    }
}

fn action(input: &Value, args: &Arguments, span: Span) -> Value {
    let value = match input {
        Value::Binary { .. } => input.clone(),
        Value::Int { val, .. } => Value::binary(
            if args.little_endian {
                val.to_le_bytes()
            } else {
                val.to_be_bytes()
            }
            .to_vec(),
            span,
        ),
        Value::Float { val, .. } => Value::binary(
            if args.little_endian {
                val.to_le_bytes()
            } else {
                val.to_be_bytes()
            }
            .to_vec(),
            span,
        ),
        Value::Filesize { val, .. } => Value::binary(
            if args.little_endian {
                val.get().to_le_bytes()
            } else {
                val.get().to_be_bytes()
            }
            .to_vec(),
            span,
        ),
        Value::String { val, .. } => Value::binary(val.as_bytes().to_vec(), span),
        Value::Bool { val, .. } => Value::binary(
            {
                let as_int = i64::from(*val);
                if args.little_endian {
                    as_int.to_le_bytes()
                } else {
                    as_int.to_be_bytes()
                }
                .to_vec()
            },
            span,
        ),
        Value::Duration { val, .. } => Value::binary(
            if args.little_endian {
                val.to_le_bytes()
            } else {
                val.to_be_bytes()
            }
            .to_vec(),
            span,
        ),
        Value::Date { val, .. } => {
            Value::binary(val.format("%c").to_string().as_bytes().to_vec(), span)
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int, float, filesize, string, date, duration, binary, or bool"
                    .into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    };

    if args.compact {
        let val_span = value.span();
        if let Value::Binary { val, .. } = value {
            let val = if args.little_endian {
                match val.iter().rposition(|&x| x != 0) {
                    Some(idx) => &val[..idx + 1],

                    // all 0s should just return a single 0 byte
                    None => &[0],
                }
            } else {
                match val.iter().position(|&x| x != 0) {
                    Some(idx) => &val[idx..],
                    None => &[0],
                }
            };

            Value::binary(val.to_vec(), val_span)
        } else {
            value
        }
    } else {
        value
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IntoBinary {})
    }

    #[rstest]
    #[case(vec![10], vec![10], vec![10])]
    #[case(vec![10, 0, 0], vec![10], vec![10, 0, 0])]
    #[case(vec![0, 0, 10], vec![0, 0, 10], vec![10])]
    #[case(vec![0, 10, 0, 0], vec![0, 10], vec![10, 0, 0])]
    fn test_compact(#[case] input: Vec<u8>, #[case] little: Vec<u8>, #[case] big: Vec<u8>) {
        let s = Value::test_binary(input);
        let actual = action(
            &s,
            &Arguments {
                cell_paths: None,
                compact: true,
                little_endian: cfg!(target_endian = "little"),
            },
            Span::test_data(),
        );
        if cfg!(target_endian = "little") {
            assert_eq!(actual, Value::test_binary(little));
        } else {
            assert_eq!(actual, Value::test_binary(big));
        }
    }
}
