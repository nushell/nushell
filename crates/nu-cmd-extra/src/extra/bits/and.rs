use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use std::iter;

#[derive(Clone)]
pub struct BitsAnd;

struct Arguments {
    target: Value,
    little_endian: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        None
    }
}

impl Command for BitsAnd {
    fn name(&self) -> &str {
        "bits and"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits and")
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
        "Performs bitwise and for ints or binary."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["logic and"]
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
                description: "Apply logical and to two numbers",
                example: "2 | bits and 2",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Apply logical and to two binary values",
                example: "0x[ab cd] | bits and 0x[99 99]",
                result: Some(Value::test_binary([0x89, 0x89])),
            },
            Example {
                description: "Apply logical and to a list of numbers",
                example: "[4 3 2] | bits and 2",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(2),
                    Value::test_int(2),
                ])),
            },
            Example {
                description: "Apply logical and to a list of binary data",
                example: "[0x[7f ff] 0x[ff f0]] | bits and 0x[99 99]",
                result: Some(Value::test_list(vec![
                    Value::test_binary([0x19, 0x99]),
                    Value::test_binary([0x99, 0x90]),
                ])),
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
        (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Value::int(lhs & rhs, span),
        (Value::Binary { val: lhs, .. }, Value::Binary { val: rhs, .. }) => {
            let max_len = lhs.len().max(rhs.len());
            let min_len = lhs.len().min(rhs.len());

            let bytes = lhs.iter().copied().zip(rhs.iter().copied());

            let pad = iter::repeat((0, 0)).take(max_len - min_len);

            let mut a;
            let mut b;

            let padded: &mut dyn Iterator<Item = (u8, u8)> = if *little_endian {
                a = pad.chain(bytes);
                &mut a
            } else {
                b = bytes.chain(pad);
                &mut b
            };

            let val: Vec<u8> = padded.map(|(l, r)| l & r).collect();

            Value::binary(val, span)
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

        test_examples(BitsAnd {})
    }
}
