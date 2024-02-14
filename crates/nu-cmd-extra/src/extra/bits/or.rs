use super::binary_op;
use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct BitsOr;

struct Arguments {
    target: Value,
    little_endian: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        None
    }
}

impl Command for BitsOr {
    fn name(&self) -> &str {
        "bits or"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits or")
            .input_output_types(vec![
                (Type::Int, Type::Int),
                (Type::Binary, Type::Binary),
                (
                    Type::List(Box::new(Type::Int)),
                    Type::List(Box::new(Type::Int)),
                ),
                (
                    Type::List(Box::new(Type::Binary)),
                    Type::List(Box::new(Type::Binary)),
                ),
            ])
            .allow_variants_without_examples(true)
            .required(
                "target",
                SyntaxShape::OneOf(vec![SyntaxShape::Binary, SyntaxShape::Int]),
                "right-hand side of the operation",
            )
            .named(
                "endian",
                SyntaxShape::String,
                "byte encode endian, available options: native(default), little, big",
                Some('e'),
            )
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
        "Performs bitwise or for ints or binary values."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["logic or"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let target: Value = call.req(engine_state, stack, 0)?;
        let endian = call.get_flag::<Value>(engine_state, stack, "endian")?;

        let little_endian = match endian {
            Some(val) => {
                let span = val.span();
                match val {
                    Value::String { val, .. } => match val.as_str() {
                        "native" => cfg!(target_endian = "little"),
                        "little" => true,
                        "big" => false,
                        _ => {
                            return Err(ShellError::TypeMismatch {
                                err_message: "Endian must be one of native, little, big"
                                    .to_string(),
                                span,
                            })
                        }
                    },
                    _ => false,
                }
            }
            None => cfg!(target_endian = "little"),
        };

        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }

        let args = Arguments {
            target,
            little_endian,
        };

        operate(action, args, input, head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Apply bits or to two numbers",
                example: "2 | bits or 6",
                result: Some(Value::test_int(6)),
            },
            Example {
                description: "Apply bitwise or to a list of numbers",
                example: "[8 3 2] | bits or 2",
                result: Some(Value::list(
                    vec![Value::test_int(10), Value::test_int(3), Value::test_int(2)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply bitwise or to binary data",
                example: "0x[88 cc] | bits or 0x[42 32]",
                result: Some(Value::binary(vec![0xca, 0xfe], Span::test_data())),
            },
            Example {
                description:
                    "Apply bitwise or to binary data of varying lengths with specified endianness",
                example: "0x[c0 ff ee] | bits or 0x[aa] --endian big",
                result: Some(Value::test_binary(vec![0xea, 0xff, 0xee])),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, span: Span) -> Value {
    let Arguments {
        target,
        little_endian,
    } = args;
    match (input, target) {
        (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Value::int(lhs | rhs, span),
        (Value::Binary { val: lhs, .. }, Value::Binary { val: rhs, .. }) => {
            Value::binary(binary_op(lhs, rhs, *little_endian, |(l, r)| l | r), span)
        }
        (Value::Binary { .. }, Value::Int { .. }) | (Value::Int { .. }, Value::Binary { .. }) => {
            Value::error(
                ShellError::PipelineMismatch {
                    exp_input_type: "input, and argument, to be both int or both binary"
                        .to_string(),
                    dst_span: target.span(),
                    src_span: span,
                },
                span,
            )
        }
        (other, Value::Int { .. } | Value::Binary { .. }) | (_, other) => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int or binary".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: other.span(),
                src_span: span,
            },
            span,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BitsOr {})
    }
}
