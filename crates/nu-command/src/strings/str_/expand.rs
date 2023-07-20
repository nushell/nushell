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

    fn extra_usage(&self) -> &str {
        "This syntax may seem familiar with `glob {A,B}.C`. The difference is glob relies on filesystem, but str expand is not. Inside braces, we put variants. Then basically we're creating all possible outcomes."
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
                description: "Export comma separated values inside braces (`{}`) to a string list.",
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
                description: "Brace expressions can be used one after another.",
                example: "\"A{b,c}D{e,f}G\" | str expand",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("AbDeG"),
                        Value::test_string("AbDfG"),
                        Value::test_string("AcDeG"),
                        Value::test_string("AcDfG"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Collection may include an empty item. It can be put at the start of the list.",
                example: "\"A{,B,C}\" | str example",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("A"),
                        Value::test_string("AB"),
                        Value::test_string("AC"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Empty item can be at the end of the collection.",
                example: "\"A{B,C,}\" | str example",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("AB"),
                        Value::test_string("AC"),
                        Value::test_string("A"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Empty item can be in the middle of the collection.",
                example: "\"A{B,,C}\" | str example",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("AB"),
                        Value::test_string("A"),
                        Value::test_string("AC"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Also, it is possible to use one inside another. Here is a real-world example, that creates files:",
                example: "\"A{B{1,3},C{2,5}}D\" | str expand",
                result: Some(Value::List{
                    vals: vec![
                        Value::test_string("AB1D"),
                        Value::test_string("AB3D"),
                        Value::test_string("AC2D"),
                        Value::test_string("AC5D"),
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
                            bracoxide::ExpansionError::NumConversionFailed(s) => Value::Error { error:
                                Box::new(ShellError::GenericError("Number Conversion Failed".to_owned(), format!("Number conversion failed at {s}."), Some(value_span), Some("Expected number, found text. Range format is `{M..N}`, where M and N are numeric values representing the starting and ending limits.".to_owned()), vec![])) },
                        },
                    }
                },
                Err(e) => Value::Error { error: Box::new(
                    match e {
                        ParsingError::NoTokens => ShellError::PipelineEmpty { dst_span: value_span },
                        ParsingError::OBraExpected(s) => ShellError::GenericError("Opening Brace Expected".to_owned(), format!("Opening brace is expected at {s}."), Some(value_span), Some("In brace syntax, we use equal amount of opening (`{`) and closing (`}`). Please, take a look at the examples.".to_owned()), vec![]),
                        ParsingError::CBraExpected(s) => ShellError::GenericError("Closing Brace Expected".to_owned(), format!("Closing brace is expected at {s}."), Some(value_span), Some("In brace syntax, we use equal amount of opening (`{`) and closing (`}`). Please, see the examples.".to_owned()), vec![]),
                        ParsingError::RangeStartLimitExpected(s) => ShellError::GenericError("Range Start Expected".to_owned(), format!("Range start limit is missing, expected at {s}."), Some(value_span), Some("In brace syntax, Range is defined like `{X..Y}`, where X and Y are a number. X is the start, Y is the end. Please, inspect the examples for more information.".to_owned()), vec![]),
                        ParsingError::RangeEndLimitExpected(s) => ShellError::GenericError("Range Start Expected".to_owned(), format!("Range start limit is missing, expected at {s}."), Some(value_span), Some("In brace syntax, Range is defined like `{X..Y}`, where X and Y are a number. X is the start, Y is the end. Please see the examples, for more information.".to_owned()), vec![]),
                        ParsingError::ExpectedText(s) => ShellError::GenericError("Expected Text".to_owned(), format!("Expected text at {s}."), Some(value_span), Some("Texts are only allowed before opening brace (`{`), after closing brace (`}`), or inside `{}`. Please take a look at the examples.".to_owned()), vec![]),
                        ParsingError::InvalidCommaUsage(s) => ShellError::GenericError("Invalid Comma Usage".to_owned(), format!("Found comma at {s}. Commas are only valid inside collection (`{{X,Y}}`)."), Some(value_span), Some("To escape comma use backslash `\\,`.".to_owned()), vec![]),
                        ParsingError::RangeCantHaveText(s) => ShellError::GenericError("Range Can not Have Text".to_owned(), format!("Expecting, brace, number, or range operator, but found text at {s}."), Some(value_span), Some("Please use the format {M..N} for ranges in brace expansion, where M and N are numeric values representing the starting and ending limits of the sequence, respectively.".to_owned()), vec![]),
                        ParsingError::ExtraRangeOperator(s) => ShellError::GenericError("Extra Range Operator".to_owned(), format!("Found additional, range operator at {s}."), Some(value_span), Some("Please, use the format `{M..N}` where M and N are numeric values representing the starting and ending limits of the range.".to_owned()), vec![]),
                        ParsingError::ExtraCBra(s) => ShellError::GenericError("Extra Closing Brace".to_owned(), format!("Used extra closing brace at {s}."), Some(value_span), Some("To escape closing brace use backslash, e.g. `\\}`".to_owned()), vec![]),
                        ParsingError::ExtraOBra(s) => ShellError::GenericError("Extra Opening Brace".to_owned(), format!("Used extra opening brace at {s}."), Some(value_span), Some("To escape opening brace use backslash, e.g. `\\{`".to_owned()), vec![]),
                        ParsingError::NothingInBraces(s) => ShellError::GenericError("Nothing In Braces".to_owned(), format!("Nothing found inside braces at {s}."), Some(value_span), Some("Please provide valid content within the braces. Additionally, you can safely remove it, not needed.".to_owned()), vec![]),
                    }
                ) }
            }
        },
        Err(e) => match e {
            TokenizationError::EmptyContent => Value::Error {
                error: Box::new(ShellError::PipelineEmpty { dst_span: value_span }),
            },
            TokenizationError::FormatNotSupported => Value::Error {
                error: Box::new(
                    ShellError::GenericError(
                        "Format Not Supported".to_owned(),
                        "Usage of only `{` or `}`. Brace Expansion syntax, needs to have equal amount of opening (`{`) and closing (`}`)".to_owned(),
                        Some(value_span),
                        Some("In brace expansion syntax, it is important to have an equal number of opening (`{`) and closing (`}`) braces. Please ensure that you provide a balanced pair of braces in your brace expansion pattern.".to_owned()),
                        vec![]
                ))
            },
            TokenizationError::NoBraces => Value::Error {
                error: Box::new(ShellError::GenericError("No Braces".to_owned(), "At least one `{}` brace expansion expected.".to_owned(), Some(value_span), Some("Please, examine the examples.".to_owned()), vec![]))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;
        test_examples(SubCommand {})
    }
}
