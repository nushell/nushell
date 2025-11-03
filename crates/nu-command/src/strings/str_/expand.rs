use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrExpand;

impl Command for StrExpand {
    fn name(&self) -> &str {
        "str expand"
    }

    fn description(&self) -> &str {
        "Generates all possible combinations defined in brace expansion syntax."
    }

    fn extra_description(&self) -> &str {
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

    fn examples(&self) -> Vec<nu_protocol::Example<'_>> {
        vec![
            Example {
                description: "Define a range inside braces to produce a list of string.",
                example: "\"{3..5}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("3"),
                        Value::test_string("4"),
                        Value::test_string("5"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Ignore the next character after the backslash ('\\')",
                example: "'A{B\\,,C}' | str expand",
                result: Some(Value::list(
                    vec![Value::test_string("AB,"), Value::test_string("AC")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Commas that are not inside any braces need to be skipped.",
                example: "'Welcome\\, {home,mon ami}!' | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("Welcome, home!"),
                        Value::test_string("Welcome, mon ami!"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Use double backslashes to add a backslash.",
                example: "'A{B\\\\,C}' | str expand",
                result: Some(Value::list(
                    vec![Value::test_string("AB\\"), Value::test_string("AC")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Export comma separated values inside braces (`{}`) to a string list.",
                example: "\"{apple,banana,cherry}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("apple"),
                        Value::test_string("banana"),
                        Value::test_string("cherry"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "If the piped data is path, you may want to use --path flag, or else manually replace the backslashes with double backslashes.",
                example: "'C:\\{Users,Windows}' | str expand --path",
                result: Some(Value::list(
                    vec![
                        Value::test_string("C:\\Users"),
                        Value::test_string("C:\\Windows"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Brace expressions can be used one after another.",
                example: "\"A{b,c}D{e,f}G\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("AbDeG"),
                        Value::test_string("AbDfG"),
                        Value::test_string("AcDeG"),
                        Value::test_string("AcDfG"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Collection may include an empty item. It can be put at the start of the list.",
                example: "\"A{,B,C}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("A"),
                        Value::test_string("AB"),
                        Value::test_string("AC"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Empty item can be at the end of the collection.",
                example: "\"A{B,C,}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("AB"),
                        Value::test_string("AC"),
                        Value::test_string("A"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Empty item can be in the middle of the collection.",
                example: "\"A{B,,C}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("AB"),
                        Value::test_string("A"),
                        Value::test_string("AC"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Also, it is possible to use one inside another. Here is a real-world example, that creates files:",
                example: "\"A{B{1,3},C{2,5}}D\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("AB1D"),
                        Value::test_string("AB3D"),
                        Value::test_string("AC2D"),
                        Value::test_string("AC5D"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Supports zero padding in numeric ranges.",
                example: "\"A{08..10}B{11..013}C\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("A08B011C"),
                        Value::test_string("A08B012C"),
                        Value::test_string("A08B013C"),
                        Value::test_string("A09B011C"),
                        Value::test_string("A09B012C"),
                        Value::test_string("A09B013C"),
                        Value::test_string("A10B011C"),
                        Value::test_string("A10B012C"),
                        Value::test_string("A10B013C"),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let is_path = call.has_flag(engine_state, stack, "path")?;
        run(call, input, is_path, engine_state)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let is_path = call.has_flag_const(working_set, "path")?;
        run(call, input, is_path, working_set.permanent())
    }
}

fn run(
    call: &Call,
    input: PipelineData,
    is_path: bool,
    engine_state: &EngineState,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    if let PipelineData::Empty = input {
        return Err(ShellError::PipelineEmpty { dst_span: span });
    }
    input.map(
        move |v| {
            let value_span = v.span();
            let type_ = v.get_type();
            match v.coerce_into_string() {
                Ok(s) => {
                    let contents = if is_path { s.replace('\\', "\\\\") } else { s };
                    str_expand(&contents, span, value_span)
                }
                Err(_) => Value::error(
                    ShellError::OnlySupportsThisInputType {
                        exp_input_type: "string".into(),
                        wrong_type: type_.to_string(),
                        dst_span: span,
                        src_span: value_span,
                    },
                    span,
                ),
            }
        },
        engine_state.signals(),
    )
}

fn str_expand(contents: &str, span: Span, value_span: Span) -> Value {
    use bracoxide::{
        expand,
        parser::{ParsingError, parse},
        tokenizer::{TokenizationError, tokenize},
    };
    match tokenize(contents) {
        Ok(tokens) => {
            match parse(&tokens) {
                Ok(node) => {
                    match expand(&node) {
                        Ok(possibilities) => {
                            Value::list(possibilities.iter().map(|e| Value::string(e,span)).collect::<Vec<Value>>(), span)
                        },
                        Err(e) => match e {
                            bracoxide::ExpansionError::NumConversionFailed(s) => Value::error(
                                ShellError::GenericError{error: "Number Conversion Failed".into(), msg: format!("Number conversion failed at {s}."), span: Some(value_span), help: Some("Expected number, found text. Range format is `{M..N}`, where M and N are numeric values representing the starting and ending limits.".into()), inner: vec![]},
                            span,
                        ),
                        },
                    }
                },
                Err(e) => Value::error(
                    match e {
                        ParsingError::NoTokens => ShellError::PipelineEmpty { dst_span: value_span },
                        ParsingError::OBraExpected(s) => ShellError::GenericError{ error: "Opening Brace Expected".into(), msg: format!("Opening brace is expected at {s}."), span: Some(value_span), help: Some("In brace syntax, we use equal amount of opening (`{`) and closing (`}`). Please, take a look at the examples.".into()), inner: vec![]},
                        ParsingError::CBraExpected(s) => ShellError::GenericError{ error: "Closing Brace Expected".into(), msg: format!("Closing brace is expected at {s}."), span: Some(value_span), help: Some("In brace syntax, we use equal amount of opening (`{`) and closing (`}`). Please, see the examples.".into()), inner: vec![]},
                        ParsingError::RangeStartLimitExpected(s) => ShellError::GenericError{error: "Range Start Expected".into(), msg: format!("Range start limit is missing, expected at {s}."), span: Some(value_span), help: Some("In brace syntax, Range is defined like `{X..Y}`, where X and Y are a number. X is the start, Y is the end. Please, inspect the examples for more information.".into()), inner: vec![]},
                        ParsingError::RangeEndLimitExpected(s) => ShellError::GenericError{ error: "Range Start Expected".into(), msg: format!("Range start limit is missing, expected at {s}."),span:  Some(value_span), help: Some("In brace syntax, Range is defined like `{X..Y}`, where X and Y are a number. X is the start, Y is the end. Please see the examples, for more information.".into()), inner: vec![]},
                        ParsingError::ExpectedText(s) => ShellError::GenericError { error: "Expected Text".into(), msg: format!("Expected text at {s}."), span: Some(value_span), help: Some("Texts are only allowed before opening brace (`{`), after closing brace (`}`), or inside `{}`. Please take a look at the examples.".into()), inner: vec![] },
                        ParsingError::InvalidCommaUsage(s) => ShellError::GenericError { error: "Invalid Comma Usage".into(), msg: format!("Found comma at {s}. Commas are only valid inside collection (`{{X,Y}}`)."),span:  Some(value_span), help: Some("To escape comma use backslash `\\,`.".into()), inner: vec![] },
                        ParsingError::RangeCantHaveText(s) => ShellError::GenericError { error: "Range Can not Have Text".into(), msg: format!("Expecting, brace, number, or range operator, but found text at {s}."), span: Some(value_span), help: Some("Please use the format {M..N} for ranges in brace expansion, where M and N are numeric values representing the starting and ending limits of the sequence, respectively.".into()), inner: vec![]},
                        ParsingError::ExtraRangeOperator(s) => ShellError::GenericError { error: "Extra Range Operator".into(), msg: format!("Found additional, range operator at {s}."), span: Some(value_span), help: Some("Please, use the format `{M..N}` where M and N are numeric values representing the starting and ending limits of the range.".into()), inner: vec![] },
                        ParsingError::ExtraCBra(s) => ShellError::GenericError { error: "Extra Closing Brace".into(), msg: format!("Used extra closing brace at {s}."), span: Some(value_span), help: Some("To escape closing brace use backslash, e.g. `\\}`".into()), inner: vec![] },
                        ParsingError::ExtraOBra(s) => ShellError::GenericError { error: "Extra Opening Brace".into(), msg: format!("Used extra opening brace at {s}."), span: Some(value_span), help: Some("To escape opening brace use backslash, e.g. `\\{`".into()), inner: vec![] },
                        ParsingError::NothingInBraces(s) => ShellError::GenericError { error: "Nothing In Braces".into(), msg: format!("Nothing found inside braces at {s}."), span: Some(value_span), help: Some("Please provide valid content within the braces. Additionally, you can safely remove it, not needed.".into()), inner: vec![] },
                    }
                ,
                span,
                )
            }
        },
        Err(e) => match e {
            TokenizationError::EmptyContent => Value::error(
                ShellError::PipelineEmpty { dst_span: value_span },
                value_span,
            ),
            TokenizationError::FormatNotSupported => Value::error(

                    ShellError::GenericError {
                        error: "Format Not Supported".into(),
                        msg: "Usage of only `{` or `}`. Brace Expansion syntax, needs to have equal amount of opening (`{`) and closing (`}`)".into(),
                        span: Some(value_span),
                        help: Some("In brace expansion syntax, it is important to have an equal number of opening (`{`) and closing (`}`) braces. Please ensure that you provide a balanced pair of braces in your brace expansion pattern.".into()),
                        inner: vec![]
                },
                 value_span,
            ),
            TokenizationError::NoBraces => Value::error(
                ShellError::GenericError { error: "No Braces".into(), msg: "At least one `{}` brace expansion expected.".into(), span: Some(value_span), help: Some("Please, examine the examples.".into()), inner: vec![] },
                value_span,
            )
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_dots_outside_curly() {
        assert_eq!(
            str_expand("..{a,b}..", Span::test_data(), Span::test_data()),
            Value::list(
                vec![
                    Value::string(String::from("..a.."), Span::test_data(),),
                    Value::string(String::from("..b.."), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
    }

    #[test]
    fn test_outer_single_item() {
        assert_eq!(
            str_expand("{W{x,y}}", Span::test_data(), Span::test_data()),
            Value::list(
                vec![
                    Value::string(String::from("Wx"), Span::test_data(),),
                    Value::string(String::from("Wy"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
    }

    #[test]
    fn dots() {
        assert_eq!(
            str_expand("{a.b.c,d}", Span::test_data(), Span::test_data()),
            Value::list(
                vec![
                    Value::string(String::from("a.b.c"), Span::test_data(),),
                    Value::string(String::from("d"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
        assert_eq!(
            str_expand("{1.2.3,a}", Span::test_data(), Span::test_data()),
            Value::list(
                vec![
                    Value::string(String::from("1.2.3"), Span::test_data(),),
                    Value::string(String::from("a"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
        assert_eq!(
            str_expand("{a-1.2,b}", Span::test_data(), Span::test_data()),
            Value::list(
                vec![
                    Value::string(String::from("a-1.2"), Span::test_data(),),
                    Value::string(String::from("b"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
    }

    #[test]
    fn test_numbers_proceeding_escape_char_not_ignored() {
        assert_eq!(
            str_expand("1\\\\{a,b}", Span::test_data(), Span::test_data()),
            Value::list(
                vec![
                    Value::string(String::from("1\\a"), Span::test_data(),),
                    Value::string(String::from("1\\b"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;
        test_examples(StrExpand {})
    }
}
