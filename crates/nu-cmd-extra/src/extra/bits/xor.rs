use super::binary_op;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct BitsXor;

impl Command for BitsXor {
    fn name(&self) -> &str {
        "bits xor"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits xor")
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
        "Performs bitwise xor for ints or binary values."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["logic xor"]
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
            move |value| binary_op(&value, &target, little_endian, |(l, r)| l ^ r, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Apply bits xor to two numbers",
                example: "2 | bits xor 2",
                result: Some(Value::test_int(0)),
            },
            Example {
                description: "Apply bitwise xor to a list of numbers",
                example: "[8 3 2] | bits xor 2",
                result: Some(Value::test_list(vec![
                    Value::test_int(10),
                    Value::test_int(1),
                    Value::test_int(0),
                ])),
            },
            Example {
                description: "Apply bitwise xor to binary data",
                example: "0x[ca fe] | bits xor 0x[ba be]",
                result: Some(Value::test_binary(vec![0x70, 0x40])),
            },
            Example {
                description: "Apply bitwise xor to binary data of varying lengths with specified endianness",
                example: "0x[ca fe] | bits xor 0x[aa] --endian big",
                result: Some(Value::test_binary(vec![0xca, 0x54])),
            },
            Example {
                description: "Apply bitwise xor to input binary data smaller than the operand",
                example: "0x[ff] | bits xor 0x[12 34 56] --endian little",
                result: Some(Value::test_binary(vec![0xed, 0x34, 0x56])),
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

        test_examples(BitsXor {})
    }
}
