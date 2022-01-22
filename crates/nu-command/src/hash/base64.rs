use base64::{decode_config, encode_config};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Base64Config {
    pub character_set: String,
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
            "character_set",
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
                example: "echo 'username:password' | hash base64 --character_set binhex --encode",
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
        call.get_flag(engine_state, stack, "character_set")?;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    if encode && decode {
        return Err(ShellError::SpannedLabeledError(
            "only one of --decode and --encode flags can be used".to_string(),
            "conflicting flags".to_string(),
            head,
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
        Some(inner_tag) => inner_tag.item,
        None => "standard".to_string(),
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
    match input {
        Value::String { val, span } => {
            let base64_config_enum: base64::Config = if &base64_config.character_set == "standard" {
                base64::STANDARD
            } else if &base64_config.character_set == "standard-no-padding" {
                base64::STANDARD_NO_PAD
            } else if &base64_config.character_set == "url-safe" {
                base64::URL_SAFE
            } else if &base64_config.character_set == "url-safe-no-padding" {
                base64::URL_SAFE_NO_PAD
            } else if &base64_config.character_set == "binhex" {
                base64::BINHEX
            } else if &base64_config.character_set == "bcrypt" {
                base64::BCRYPT
            } else if &base64_config.character_set == "crypt" {
                base64::CRYPT
            } else {
                return Err(ShellError::SpannedLabeledError(
                    "value is not an accepted character set".to_string(),
                    format!(
                        "{} is not a valid character-set.\nPlease use `help hash base64` to see a list of valid character sets.",
                        &base64_config.character_set
                    ),
                    *span,
                ));
            };

            match base64_config.action_type {
                ActionType::Encode => Ok(Value::string(
                    encode_config(&val, base64_config_enum),
                    *command_span,
                )),

                ActionType::Decode => {
                    let decode_result = decode_config(&val, base64_config_enum);

                    match decode_result {
                        Ok(decoded_value) => Ok(Value::string(
                            std::string::String::from_utf8_lossy(&decoded_value),
                            *command_span,
                        )),
                        Err(_) => Err(ShellError::SpannedLabeledError(
                            "value could not be base64 decoded".to_string(),
                            format!(
                                "invalid base64 input for character set {}",
                                &base64_config.character_set
                            ),
                            *command_span,
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
    use super::{action, ActionType, Base64Config};
    use nu_protocol::{Span, Value};

    #[test]
    fn base64_encode_standard() {
        let word = Value::string("username:password", Span::test_data());
        let expected = Value::string("dXNlcm5hbWU6cGFzc3dvcmQ=", Span::test_data());

        let actual = action(
            &word,
            &Base64Config {
                character_set: "standard".to_string(),
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
                character_set: "standard-no-padding".to_string(),
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
                character_set: "url-safe".to_string(),
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
                character_set: "binhex".to_string(),
                action_type: ActionType::Decode,
            },
            &Span::test_data(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}
