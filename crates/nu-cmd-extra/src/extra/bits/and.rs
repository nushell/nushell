use std::iter;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct BitsAnd;

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
                SyntaxShape::OneOf(vec![
                    SyntaxShape::Binary,
                    SyntaxShape::Int
                ]),
                "right-hand side of the operation")
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

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }

        match target {
            Value::Int { val: rhs, ..} => input.map(
                move |value| and_int(value, rhs, head),
                engine_state.ctrlc.clone(),
            ),
            Value::Binary { val: rhs, ..} => input.map(
                move |value| and_bytes(value, rhs.clone(), little_endian, head),
                engine_state.ctrlc.clone(),
            ),
            other => Err(
                ShellError::OnlySupportsThisInputType {
                    exp_input_type: "int or binary".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: other.span(),
                },
            )
        }
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
                result: Some(Value::test_binary([137, 137]))
            },
            Example {
                description: "Apply logical and to a list of numbers",
                example: "[4 3 2] | bits and 2",
                result: Some(Value::test_list(
                    vec![Value::test_int(0), Value::test_int(2), Value::test_int(2)],
                )),
            },
            Example {
                description: "Apply logical and to a list of binary data",
                example: "[0x[7f ff] 0x[ff f0]] | bits and 0x[99 99]",
                result: Some(Value::test_list(
                    vec![Value::test_binary([25, 153]), Value::test_binary([153, 144])],
                )),
            },
        ]
    }
}

fn and_int(
    value: Value,
    target: i64,
    head: Span
) -> Value {
    let span = value.span();
    match value {
        Value::Int { val, .. } => Value::int(val & target, span),
        Value::Error { .. } => value,
        Value::Binary { .. } => Value::error(
            ShellError::PipelineMismatch {
                exp_input_type: "input, and argument, to be both int or both binary"
                    .to_string(),
                dst_span: head,
                src_span: span,
            },
            head
        ),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int or binary".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}
    
fn and_bytes(
    value: Value,
    target: Vec<u8>,
    little_endian: bool,
    head: Span
) -> Value {
    let span = value.span();
    match value {
        Value::Binary { val: lhs, .. } => {
            let rhs = target;
            let max_len = lhs.len().max(rhs.len());
            let min_len = lhs.len().min(rhs.len());

            let bytes = lhs.into_iter().zip(rhs);

            let pad = iter::repeat((0, 0))
                .take(max_len - min_len);

            let mut a;
            let mut b;

            let padded: &mut dyn Iterator<Item = (u8, u8)> = if little_endian {
                a = pad.chain(bytes);
                &mut a
            } else {
                b = bytes.chain(pad);
                &mut b
            };

            let val: Vec<u8> = padded.map(|(l, r)| l & r).collect();
    
            Value::binary(val, span)
        },
        Value::Error { .. } => value,
        Value::Int { .. } => Value::error(
            ShellError::PipelineMismatch {
                exp_input_type: "input, and argument, to be both int or both binary"
                    .to_string(),
                dst_span: head,
                src_span: span,
            },
            head
        ),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int or binary".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
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
