use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
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
            .input_output_types(vec![
                (Type::String, Type::List(Box::new(Type::String))),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::List(Box::new(Type::String)))),
                ),
            ])
            .switch(
                "path",
                "Replaces all backslashes with double backslashes, useful for Path.",
                None,
            )
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "Define a range inside braces to produce a list of string.",
                example: "\"{3..5}\" | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("3"),
                        SpannedValue::test_string("4"),
                        SpannedValue::test_string("5")
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Ignore the next character after the backslash ('\\')",
                example: "'A{B\\,,C}' | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("AB,"),
                        SpannedValue::test_string("AC"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Commas that are not inside any braces need to be skipped.",
                example: "'Welcome\\, {home,mon ami}!' | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("Welcome, home!"),
                        SpannedValue::test_string("Welcome, mon ami!"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Use double backslashes to add a backslash.",
                example: "'A{B\\\\,C}' | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("AB\\"),
                        SpannedValue::test_string("AC"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Export comma separated values inside braces (`{}`) to a string list.",
                example: "\"{apple,banana,cherry}\" | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("apple"),
                        SpannedValue::test_string("banana"),
                        SpannedValue::test_string("cherry")
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "If the piped data is path, you may want to use --path flag, or else manually replace the backslashes with double backslashes.",
                example: "'C:\\{Users,Windows}' | str expand --path",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("C:\\Users"),
                        SpannedValue::test_string("C:\\Windows"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Brace expressions can be used one after another.",
                example: "\"A{b,c}D{e,f}G\" | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("AbDeG"),
                        SpannedValue::test_string("AbDfG"),
                        SpannedValue::test_string("AcDeG"),
                        SpannedValue::test_string("AcDfG"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Collection may include an empty item. It can be put at the start of the list.",
                example: "\"A{,B,C}\" | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("A"),
                        SpannedValue::test_string("AB"),
                        SpannedValue::test_string("AC"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Empty item can be at the end of the collection.",
                example: "\"A{B,C,}\" | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("AB"),
                        SpannedValue::test_string("AC"),
                        SpannedValue::test_string("A"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Empty item can be in the middle of the collection.",
                example: "\"A{B,,C}\" | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("AB"),
                        SpannedValue::test_string("A"),
                        SpannedValue::test_string("AC"),
                    ],
                    span: Span::test_data()
                },)
            },

            Example {
                description: "Also, it is possible to use one inside another. Here is a real-world example, that creates files:",
                example: "\"A{B{1,3},C{2,5}}D\" | str expand",
                result: Some(SpannedValue::List{
                    vals: vec![
                        SpannedValue::test_string("AB1D"),
                        SpannedValue::test_string("AB3D"),
                        SpannedValue::test_string("AC2D"),
                        SpannedValue::test_string("AC5D"),
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
        let is_path = call.has_flag("path");
        input.map(
            move |v| {
                let value_span = match v.span() {
                    Err(v) => return SpannedValue::Error { error: Box::new(v) },
                    Ok(v) => v,
                };
                match v.as_string() {
                    Ok(s) => {
                        let contents = if is_path { s.replace('\\', "\\\\") } else { s };
                        str_expand(&contents, span, v.expect_span())
                    }
                    Err(_) => SpannedValue::Error {
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

fn str_expand(contents: &str, span: Span, value_span: Span) -> SpannedValue {
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
                            SpannedValue::List { vals: possibilities.iter().map(|e| SpannedValue::string(e,span)).collect::<Vec<SpannedValue>>(), span }
                        },
                        Err(e) => match e {
                            bracoxide::ExpansionError::NumConversionFailed(s) => SpannedValue::Error { error:
                                Box::new(ShellError::GenericError("Number Conversion Failed".to_owned(), format!("Number conversion failed at {s}."), Some(value_span), Some("Expected number, found text. Range format is `{M..N}`, where M and N are numeric values representing the starting and ending limits.".to_owned()), vec![])) },
                        },
                    }
                },
                Err(e) => SpannedValue::Error { error: Box::new(
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
            TokenizationError::EmptyContent => SpannedValue::Error {
                error: Box::new(ShellError::PipelineEmpty { dst_span: value_span }),
            },
            TokenizationError::FormatNotSupported => SpannedValue::Error {
                error: Box::new(
                    ShellError::GenericError(
                        "Format Not Supported".to_owned(),
                        "Usage of only `{` or `}`. Brace Expansion syntax, needs to have equal amount of opening (`{`) and closing (`}`)".to_owned(),
                        Some(value_span),
                        Some("In brace expansion syntax, it is important to have an equal number of opening (`{`) and closing (`}`) braces. Please ensure that you provide a balanced pair of braces in your brace expansion pattern.".to_owned()),
                        vec![]
                ))
            },
            TokenizationError::NoBraces => SpannedValue::Error {
                error: Box::new(ShellError::GenericError("No Braces".to_owned(), "At least one `{}` brace expansion expected.".to_owned(), Some(value_span), Some("Please, examine the examples.".to_owned()), vec![]))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dots() {
        assert_eq!(
            str_expand("{a.b.c,d}", Span::test_data(), Span::test_data()),
            SpannedValue::List {
                vals: vec![
                    SpannedValue::String {
                        val: String::from("a.b.c"),
                        span: Span::test_data(),
                    },
                    SpannedValue::String {
                        val: String::from("d"),
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }
        );
        assert_eq!(
            str_expand("{1.2.3,a}", Span::test_data(), Span::test_data()),
            SpannedValue::List {
                vals: vec![
                    SpannedValue::String {
                        val: String::from("1.2.3"),
                        span: Span::test_data(),
                    },
                    SpannedValue::String {
                        val: String::from("a"),
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }
        );
        assert_eq!(
            str_expand("{a-1.2,b}", Span::test_data(), Span::test_data()),
            SpannedValue::List {
                vals: vec![
                    SpannedValue::String {
                        val: String::from("a-1.2"),
                        span: Span::test_data(),
                    },
                    SpannedValue::String {
                        val: String::from("b"),
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }
        );
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;
        test_examples(SubCommand {})
    }
}
