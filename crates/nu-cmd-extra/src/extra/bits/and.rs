use super::binary_op;
use nu_engine::command_prelude::*;

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
                SyntaxShape::OneOf(vec![SyntaxShape::Binary, SyntaxShape::Int]),
                "Right-hand side of the operation.",
            )
            .named(
                "endian",
                SyntaxShape::String,
                "byte encode endian, available options: native(default), little, big",
                Some('e'),
            )
            .category(Category::Bits)
    }

    fn description(&self) -> &str {
        "Performs bitwise and for ints or binary values."
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

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }

        input.map(
            move |value| binary_op(&value, &target, little_endian, |(l, r)| l & r, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Apply bitwise and to two numbers",
                example: "2 | bits and 2",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Apply bitwise and to two binary values",
                example: "0x[ab cd] | bits and 0x[99 99]",
                result: Some(Value::test_binary([0x89, 0x89])),
            },
            Example {
                description: "Apply bitwise and to a list of numbers",
                example: "[4 3 2] | bits and 2",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(2),
                    Value::test_int(2),
                ])),
            },
            Example {
                description: "Apply bitwise and to a list of binary data",
                example: "[0x[7f ff] 0x[ff f0]] | bits and 0x[99 99]",
                result: Some(Value::test_list(vec![
                    Value::test_binary([0x19, 0x99]),
                    Value::test_binary([0x99, 0x90]),
                ])),
            },
            Example {
                description: "Apply bitwise and to binary data of varying lengths with specified endianness",
                example: "0x[c0 ff ee] | bits and 0x[ff] --endian big",
                result: Some(Value::test_binary(vec![0x00, 0x00, 0xee])),
            },
            Example {
                description: "Apply bitwise and to input binary data smaller than the operand",
                example: "0x[ff] | bits and 0x[12 34 56] --endian little",
                result: Some(Value::test_binary(vec![0x12, 0x00, 0x00])),
            },
        ]
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
