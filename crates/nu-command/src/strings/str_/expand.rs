use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str expand"
    }

    fn usage(&self) -> &str {
        "Generates all possible combinations defined in brace expansion syntax."
    }

    fn signature(&self) -> Signature {
        Signature::build("str expand")
            .input_output_types(vec![(Type::String, Type::List(Box::new(Type::String)))])
            .vectorizes_over_list(true)
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "Define a range inside braces to produce a list of string.",
                example: "\"{3..5}\" | str expand",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("3"),
                        Value::test_string("4"),
                        Value::test_string("5")
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Export comma seperated values inside braces (`{}`) to a string list.",
                example: "\"{apple,banana,cherry}\" | str expand",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("apple"),
                        Value::test_string("banana"),
                        Value::test_string("cherry")
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Instead of listing all the files that has a common path, you may want to use brace expansion syntax.",
                example: "\"~/Desktop/{file1,file2,file3}.txt\" | str expand",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("~/Desktop/file1.txt"),
                        Value::test_string("~/Desktop/file2.txt"),
                        Value::test_string("~/Desktop/file3.txt")
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Brace expressions can be used one after another.",
                example: "\"~/Videos/{Movies,Series}/{Comedy,Adventure}\" | str expand",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("~/Videos/Movies/Comedy"),
                        Value::test_string("~/Videos/Movies/Adventure"),
                        Value::test_string("~/Videos/Series/Comedy"),
                        Value::test_string("~/Videos/Series/Adventure"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Also, it is possible to use one inside another. Here is a real-world example, that creates files:",
                example: "^touch \"/etc/libvirt/hooks/{qemu,qemu.d/win11/{prepare/begin/{10,20,30}.sh,release/end/{10,20,30,40}.sh}}\" | str expand",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("/etc/libvirt/hooks/qemu"),
                        Value::test_string("/etc/libvirt/hooks/qemu.d/win11/prepare/begin/10.sh"),
                        Value::test_string("/etc/libvirt/hooks/qemu.d/win11/prepare/begin/20.sh"),
                        Value::test_string("/etc/libvirt/hooks/qemu.d/win11/prepare/begin/30.sh"),
                        Value::test_string("/etc/libvirt/hooks/qemu.d/win11/release/end/10.sh"),
                        Value::test_string("/etc/libvirt/hooks/qemu.d/win11/release/end/20.sh"),
                        Value::test_string("/etc/libvirt/hooks/qemu.d/win11/release/end/30.sh"),
                        Value::test_string("/etc/libvirt/hooks/qemu.d/win11/release/end/40.sh"),
                    ],
                    span: Span::test_data()
                },)
            }
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: span });
        }
        input.map(
            move |v| {
                let value_span = match v.span() {
                    Err(v) => return Value::Error { error: Box::new(v) },
                    Ok(v) => v,
                };
                match v.as_string() {
                    Ok(s) => str_expand(&s, span, v.expect_span()),
                    Err(_) => Value::Error {
                        error: Box::new(ShellError::PipelineMismatch {
                            exp_input_type: "string".into(),
                            dst_span: span,
                            src_span: value_span,
                        }),
                    },
                }
            },
            engine_state.ctrlc.clone(),
        )
    }
}

fn str_expand(contents: &str, span: Span, value_span: Span) -> Value {
    use bracoxide::{
        expand,
        parser::{parse, ParsingError},
        tokenizer::{tokenize, TokenizationError},
    };
    match tokenize(contents) {
        Ok(tokens) => {
            match parse(&tokens) {
                Ok(node) => {
                    match expand(&node) {
                        Ok(possibilities) => {
                            Value::List { vals: possibilities.iter().map(|e| Value::string(e,span)).collect::<Vec<Value>>(), span }
                        },
                        Err(e) => match e {
                            bracoxide::ExpansionError::NumConversionFailed(_) => todo!(),
                        },
                    }
                },
                Err(e) => match e {
                    ParsingError::NoTokens => todo!(),
                    ParsingError::OBraExpected(_) => todo!(),
                    ParsingError::CBraExpected(_) => todo!(),
                    ParsingError::RangeStartLimitExpected(_) => todo!(),
                    ParsingError::RangeEndLimitExpected(_) => todo!(),
                    ParsingError::ExpectedText(_) => todo!(),
                    ParsingError::InvalidCommaUsage(_) => todo!(),
                    ParsingError::ExtraCBra(_) => todo!(),
                    ParsingError::ExtraOBra(_) => todo!(),
                    ParsingError::NothingInBraces(_) => todo!(),
                    ParsingError::RangeCantHaveText(_) => todo!(),
                    ParsingError::ExtraRangeOperator(_) => todo!(),
                },
            }
        },
        Err(e) => match e {
            TokenizationError::EmptyContent => Value::Error {
                error: Box::new(ShellError::PipelineEmpty { dst_span: value_span }),
            },
            TokenizationError::FormatNotSupported => Value::Error { 
                error: Box::new(
                    ShellError::DelimiterError {
                    msg: "Only opening or closing brace is used. Brace Expansion syntax is as follows: `{COL,LEC,TION}`.".to_owned(),
                    span: value_span
                })
            },
            TokenizationError::NoBraces => Value::Error {
                error: Box::new(ShellError::GenericError("No Braces".to_owned(), "At least one `{}` brace expansion expected.".to_owned(), Some(value_span), Some("Please, examine the examples.".to_owned()), vec![]))
            }
        },
    }
}
