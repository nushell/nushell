use base64::{decode_config, encode_config};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Base64Config {
    pub character_set: Spanned<String>,
    pub action_type: ActionType,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActionType {
    Encode,
    Decode,
}

#[derive(Clone)]
pub struct Base64;

impl Command for Base64 {
    fn name(&self) -> &str {
        "hash base64"
    }

    fn signature(&self) -> Signature {
        Signature::build("hash base64")
        .named(
            "character-set",
            SyntaxShape::String,
            "specify the character rules for encoding the input.\n\
                    \tValid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding',\
                    'binhex', 'bcrypt', 'crypt'",
            Some('c'),
        )
        .switch(
            "encode",
            "encode the input as base64. This is the default behavior if not specified.",
            Some('e')
        )
        .switch(
            "decode",
            "decode the input from base64",
            Some('d'))
        .rest(
            "rest",
            SyntaxShape::CellPath,
            "optionally base64 encode / decode data by column paths",
        )
        .category(Category::Hash)
    }

    fn usage(&self) -> &str {
        "base64 encode or decode a value"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Base64 encode a string with default settings",
                example: "echo 'username:password' | hash base64",
                result: Some(Value::string("dXNlcm5hbWU6cGFzc3dvcmQ=", Span::test_data())),
            },
            Example {
                description: "Base64 encode a string with the binhex character set",
                example: "echo 'username:password' | hash base64 --character-set binhex --encode",
                result: Some(Value::string("F@0NEPjJD97kE'&bEhFZEP3", Span::test_data())),
            },
            Example {
                description: "Base64 decode a value",
                example: "echo 'dXNlcm5hbWU6cGFzc3dvcmQ=' | hash base64 --decode",
                result: Some(Value::string("username:password", Span::test_data())),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let encode = call.has_flag("encode");
    let decode = call.has_flag("decode");
    let character_set: Option<Spanned<String>> =
        call.get_flag(engine_state, stack, "character-set")?;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    if encode && decode {
        return Err(ShellError::GenericError(
            "only one of --decode and --encode flags can be used".to_string(),
            "conflicting flags".to_string(),
            Some(head),
            None,
            Vec::new(),
        ));
    }

    // Default the action to be encoding if no flags are specified.
    let action_type = if decode {
        ActionType::Decode
    } else {
        ActionType::Encode
    };

    // Default the character set to standard if the argument is not specified.
    let character_set = match character_set {
        Some(inner_tag) => inner_tag,
        None => Spanned {
            item: "standard".to_string(),
            span: head, // actually this span is always useless, because default character_set is always valid.
        },
    };

    let encoding_config = Base64Config {
        character_set,
        action_type,
    };

    input.map(
        move |v| {
            if column_paths.is_empty() {
                match action(&v, &encoding_config, &head) {
                    Ok(v) => v,
                    Err(e) => Value::Error { error: e },
                }
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let config = encoding_config.clone();

                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| match action(old, &config, &head) {
                            Ok(v) => v,
                            Err(e) => Value::Error { error: e },
                        }),
                    );
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn action(
    input: &Value,
    base64_config: &Base64Config,
    command_span: &Span,
) -> Result<Value, ShellError> {
    let config_character_set = &base64_config.character_set;
    let base64_config_enum: base64::Config = match config_character_set.item.as_str() {
        "standard" => base64::STANDARD,
        "standard-no-padding" => base64::STANDARD_NO_PAD,
        "url-safe" => base64::URL_SAFE,
        "url-safe-no-padding" => base64::URL_SAFE_NO_PAD,
        "binhex" => base64::BINHEX,
        "bcrypt" => base64::BCRYPT,
        "crypt" => base64::CRYPT,
        not_valid => return Err(ShellError::GenericError(
            "value is not an accepted character set".to_string(),
            format!(
                "{} is not a valid character-set.\nPlease use `help hash base64` to see a list of valid character sets.",
                not_valid
            ),
            Some(config_character_set.span),
            None,
            Vec::new(),
        ))
    };
    match input {
        Value::Binary { val, .. } => match base64_config.action_type {
            ActionType::Encode => Ok(Value::string(
                encode_config(&val, base64_config_enum),
                *command_span,
            )),
            ActionType::Decode => Err(ShellError::UnsupportedInput(
                "Binary data can only support encoding".to_string(),
                *command_span,
            )),
        },
        Value::String { val, .. } => {
            match base64_config.action_type {
                ActionType::Encode => Ok(Value::string(
                    encode_config(&val, base64_config_enum),
                    *command_span,
                )),

                ActionType::Decode => {
                    // for decode, input val may contains invalid new line character, which is ok to omitted them by default.
                    let val = val.clone();
                    let val = val.replace("\r\n", "").replace('\n', "");
                    let decode_result = decode_config(&val, base64_config_enum);

                    match decode_result {
                        Ok(decoded_value) => Ok(Value::string(
                            std::string::String::from_utf8_lossy(&decoded_value),
                            *command_span,
                        )),
                        Err(_) => Err(ShellError::GenericError(
                            "value could not be base64 decoded".to_string(),
                            format!(
                                "invalid base64 input for character set {}",
                                &config_character_set.item
                            ),
                            Some(*command_span),
                            None,
                            Vec::new(),
                        )),
                    }
                }
            }
        }
        other => Err(ShellError::TypeMismatch(
            format!("value is {}, not string", other.get_type()),
            other.span()?,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{action, ActionType, Base64, Base64Config};
    use nu_protocol::{Span, Spanned, Value};

    #[test]
    fn test_examples() {
        use crate::test_examples;
        test_examples(Base64 {})
    }

    #[test]
    fn base64_encode_standard() {
        let word = Value::string("username:password", Span::test_data());
        let expected = Value::string("dXNlcm5hbWU6cGFzc3dvcmQ=", Span::test_data());

        let actual = action(
            &word,
            &Base64Config {
                character_set: Spanned {
                    item: "standard".to_string(),
                    span: Span::test_data(),
                },
                action_type: ActionType::Encode,
            },
            &Span::test_data(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_encode_standard_no_padding() {
        let word = Value::string("username:password", Span::test_data());
        let expected = Value::string("dXNlcm5hbWU6cGFzc3dvcmQ", Span::test_data());

        let actual = action(
            &word,
            &Base64Config {
                character_set: Spanned {
                    item: "standard-no-padding".to_string(),
                    span: Span::test_data(),
                },
                action_type: ActionType::Encode,
            },
            &Span::test_data(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_encode_url_safe() {
        let word = Value::string("this is for url", Span::test_data());
        let expected = Value::string("dGhpcyBpcyBmb3IgdXJs", Span::test_data());

        let actual = action(
            &word,
            &Base64Config {
                character_set: Spanned {
                    item: "url-safe".to_string(),
                    span: Span::test_data(),
                },
                action_type: ActionType::Encode,
            },
            &Span::test_data(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_decode_binhex() {
        let word = Value::string("A5\"KC9jRB@IIF'8bF!", Span::test_data());
        let expected = Value::string("a binhex test", Span::test_data());

        let actual = action(
            &word,
            &Base64Config {
                character_set: Spanned {
                    item: "binhex".to_string(),
                    span: Span::test_data(),
                },
                action_type: ActionType::Decode,
            },
            &Span::test_data(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_decode_binhex_with_new_line_input() {
        let word = Value::string("A5\"KC9jRB\n@IIF'8bF!", Span::test_data());
        let expected = Value::string("a binhex test", Span::test_data());

        let actual = action(
            &word,
            &Base64Config {
                character_set: Spanned {
                    item: "binhex".to_string(),
                    span: Span::test_data(),
                },
                action_type: ActionType::Decode,
            },
            &Span::test_data(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_encode_binary() {
        let word = Value::Binary {
            val: vec![77, 97, 110],
            span: Span::test_data(),
        };
        let expected = Value::string("TWFu", Span::test_data());

        let actual = action(
            &word,
            &Base64Config {
                character_set: Spanned {
                    item: "standard".to_string(),
                    span: Span::test_data(),
                },
                action_type: ActionType::Encode,
            },
            &Span::test_data(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_decode_binary_expect_error() {
        let word = Value::Binary {
            val: vec![77, 97, 110],
            span: Span::test_data(),
        };

        let actual = action(
            &word,
            &Base64Config {
                character_set: Spanned {
                    item: "standard".to_string(),
                    span: Span::test_data(),
                },
                action_type: ActionType::Decode,
            },
            &Span::test_data(),
        );
        assert!(actual.is_err())
    }
}
