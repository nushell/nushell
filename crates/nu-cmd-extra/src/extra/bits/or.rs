use super::binary_op;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct BitsOr;

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
            move |value| binary_op(&value, &target, little_endian, |(l, r)| l | r, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Apply bits or to two numbers",
                example: "2 | bits or 6",
                result: Some(Value::test_int(6)),
            },
            Example {
                description: "Apply bitwise or to a list of numbers",
                example: "[8 3 2] | bits or 2",
                result: Some(Value::test_list(vec![
                    Value::test_int(10),
                    Value::test_int(3),
                    Value::test_int(2),
                ])),
            },
            Example {
                description: "Apply bitwise or to binary data",
                example: "0x[88 cc] | bits or 0x[42 32]",
                result: Some(Value::test_binary(vec![0xca, 0xfe])),
            },
            Example {
                description: "Apply bitwise or to binary data of varying lengths with specified endianness",
                example: "0x[c0 ff ee] | bits or 0x[ff] --endian big",
                result: Some(Value::test_binary(vec![0xc0, 0xff, 0xff])),
            },
            Example {
                description: "Apply bitwise or to input binary data smaller than the operor",
                example: "0x[ff] | bits or 0x[12 34 56] --endian little",
                result: Some(Value::test_binary(vec![0xff, 0x34, 0x56])),
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

        test_examples(BitsOr {})
    }
}
