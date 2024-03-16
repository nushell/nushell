use base64::{
    alphabet,
    engine::{
        general_purpose::{NO_PAD, PAD},
        GeneralPurpose,
    },
    Engine,
};
use nu_cmd_base::input_handler::{operate as general_operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{EngineState, Stack},
    PipelineData, ShellError, Span, Spanned, Value,
};

pub const CHARACTER_SET_DESC: &str = "specify the character rules for encoding the input.\n\
                    \tValid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding',\
                    'binhex', 'bcrypt', 'crypt', 'mutf7'";

#[derive(Clone)]
pub struct Base64Config {
    pub character_set: Spanned<String>,
    pub action_type: ActionType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    Encode,
    Decode,
}

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
    binary: bool,
    encoding_config: Base64Config,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

pub fn operate(
    action_type: ActionType,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let character_set: Option<Spanned<String>> =
        call.get_flag(engine_state, stack, "character-set")?;
    let binary = call.has_flag(engine_state, stack, "binary")?;
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

    // Default the character set to standard if the argument is not specified.
    let character_set = match character_set {
        Some(inner_tag) => inner_tag,
        None => Spanned {
            item: "standard".to_string(),
            span: head, // actually this span is always useless, because default character_set is always valid.
        },
    };

    let args = Arguments {
        encoding_config: Base64Config {
            character_set,
            action_type,
        },
        binary,
        cell_paths,
    };

    general_operate(action, args, input, call.head, engine_state.ctrlc.clone())
}

fn action(
    input: &Value,
    // only used for `decode` action
    args: &Arguments,
    command_span: Span,
) -> Value {
    let base64_config = &args.encoding_config;
    let output_binary = args.binary;

    let config_character_set = &base64_config.character_set;
    let base64_engine: GeneralPurpose = match config_character_set.item.as_str() {
        "standard" => GeneralPurpose::new(&alphabet::STANDARD, PAD),
        "standard-no-padding" => GeneralPurpose::new(&alphabet::STANDARD, NO_PAD),
        "url-safe" => GeneralPurpose::new(&alphabet::URL_SAFE, PAD),
        "url-safe-no-padding" => GeneralPurpose::new(&alphabet::URL_SAFE, NO_PAD),
        "bcrypt" => GeneralPurpose::new(&alphabet::BCRYPT, NO_PAD),
        "binhex" => GeneralPurpose::new(&alphabet::BIN_HEX, NO_PAD),
        "crypt" => GeneralPurpose::new(&alphabet::CRYPT, NO_PAD),
        "mutf7" => GeneralPurpose::new(&alphabet::IMAP_MUTF7, NO_PAD),
        not_valid => return Value::error (
            ShellError::GenericError {
            error: "value is not an accepted character set".into(),
            msg: format!(
                "{not_valid} is not a valid character-set.\nPlease use `help encode base64` to see a list of valid character sets."
            ),
            span: Some(config_character_set.span),
            help: None,
            inner: vec![],
        }, config_character_set.span)
    };
    let value_span = input.span();
    match input {
        // Propagate existing errors.
        Value::Error { .. } => input.clone(),
        Value::Binary { val, .. } => match base64_config.action_type {
            ActionType::Encode => {
                let mut enc_vec = vec![0; val.len() * 4 / 3 + 4];
                let bytes_written = match base64_engine.encode_slice(val, &mut enc_vec) {
                    Ok(bytes_written) => bytes_written,
                    Err(e) => {
                        return Value::error(
                            ShellError::GenericError {
                                error: "Error encoding data".into(),
                                msg: e.to_string(),
                                span: Some(value_span),
                                help: None,
                                inner: vec![],
                            },
                            value_span,
                        )
                    }
                };
                enc_vec.truncate(bytes_written);
                Value::string(std::str::from_utf8(&enc_vec).unwrap_or(""), command_span)
            }
            ActionType::Decode => Value::error(
                ShellError::UnsupportedInput {
                    msg: "Binary data can only be encoded".to_string(),
                    input: "value originates from here".into(),
                    msg_span: command_span,
                    input_span: input.span(),
                },
                command_span,
            ),
        },
        Value::String { val, .. } => {
            match base64_config.action_type {
                ActionType::Encode => {
                    let mut enc_str = String::new();
                    base64_engine.encode_string(val, &mut enc_str);
                    Value::string(enc_str, command_span)
                }

                ActionType::Decode => {
                    // for decode, input val may contains invalid new line character, which is ok to omitted them by default.
                    let val = val.clone();
                    let val = val.replace("\r\n", "").replace('\n', "");

                    match base64_engine.decode(val) {
                        Ok(decoded_value) => {
                            if output_binary {
                                Value::binary(decoded_value, command_span)
                            } else {
                                match String::from_utf8(decoded_value) {
                                    Ok(string_value) => Value::string(string_value, command_span),
                                    Err(e) => Value::error(
                                        ShellError::GenericError {
                                            error: "base64 payload isn't a valid utf-8 sequence"
                                                .into(),
                                            msg: e.to_string(),
                                            span: Some(value_span),
                                            help: Some(
                                                "consider using the `--binary` flag".to_owned(),
                                            ),
                                            inner: vec![],
                                        },
                                        value_span,
                                    ),
                                }
                            }
                        }
                        Err(_) => Value::error(
                            ShellError::GenericError {
                                error: "value could not be base64 decoded".into(),
                                msg: format!(
                                    "invalid base64 input for character set {}",
                                    &config_character_set.item
                                ),
                                span: Some(command_span),
                                help: None,
                                inner: vec![],
                            },
                            command_span,
                        ),
                    }
                }
            }
        }
        other => Value::error(
            ShellError::TypeMismatch {
                err_message: format!("string or binary, not {}", other.get_type()),
                span: other.span(),
            },
            other.span(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{action, ActionType, Arguments, Base64Config};
    use nu_protocol::{Span, Spanned, Value};

    #[test]
    fn base64_encode_standard() {
        let word = Value::test_string("Some Data Padding");
        let expected = Value::test_string("U29tZSBEYXRhIFBhZGRpbmc=");

        let actual = action(
            &word,
            &Arguments {
                encoding_config: Base64Config {
                    character_set: Spanned {
                        item: "standard".to_string(),
                        span: Span::test_data(),
                    },
                    action_type: ActionType::Encode,
                },
                binary: true,
                cell_paths: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_encode_standard_no_padding() {
        let word = Value::test_string("Some Data Padding");
        let expected = Value::test_string("U29tZSBEYXRhIFBhZGRpbmc");

        let actual = action(
            &word,
            &Arguments {
                encoding_config: Base64Config {
                    character_set: Spanned {
                        item: "standard-no-padding".to_string(),
                        span: Span::test_data(),
                    },
                    action_type: ActionType::Encode,
                },
                binary: true,
                cell_paths: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_encode_url_safe() {
        let word = Value::test_string("this is for url");
        let expected = Value::test_string("dGhpcyBpcyBmb3IgdXJs");

        let actual = action(
            &word,
            &Arguments {
                encoding_config: Base64Config {
                    character_set: Spanned {
                        item: "url-safe".to_string(),
                        span: Span::test_data(),
                    },
                    action_type: ActionType::Encode,
                },
                binary: true,
                cell_paths: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_decode_binhex() {
        let word = Value::test_string("A5\"KC9jRB@IIF'8bF!");
        let expected = Value::binary(b"a binhex test".as_slice(), Span::test_data());

        let actual = action(
            &word,
            &Arguments {
                encoding_config: Base64Config {
                    character_set: Spanned {
                        item: "binhex".to_string(),
                        span: Span::test_data(),
                    },
                    action_type: ActionType::Decode,
                },
                binary: true,
                cell_paths: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_decode_binhex_with_new_line_input() {
        let word = Value::test_string("A5\"KC9jRB\n@IIF'8bF!");
        let expected = Value::binary(b"a binhex test".as_slice(), Span::test_data());

        let actual = action(
            &word,
            &Arguments {
                encoding_config: Base64Config {
                    character_set: Spanned {
                        item: "binhex".to_string(),
                        span: Span::test_data(),
                    },
                    action_type: ActionType::Decode,
                },
                binary: true,
                cell_paths: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_encode_binary() {
        let word = Value::binary(vec![77, 97, 110], Span::test_data());
        let expected = Value::test_string("TWFu");

        let actual = action(
            &word,
            &Arguments {
                encoding_config: Base64Config {
                    character_set: Spanned {
                        item: "standard".to_string(),
                        span: Span::test_data(),
                    },
                    action_type: ActionType::Encode,
                },
                binary: true,
                cell_paths: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_decode_binary_expect_error() {
        let word = Value::binary(vec![77, 97, 110], Span::test_data());

        let actual = action(
            &word,
            &Arguments {
                encoding_config: Base64Config {
                    character_set: Spanned {
                        item: "standard".to_string(),
                        span: Span::test_data(),
                    },
                    action_type: ActionType::Decode,
                },
                binary: true,
                cell_paths: None,
            },
            Span::test_data(),
        );

        match actual {
            Value::Error { .. } => {}
            _ => panic!("the result should be Value::Error"),
        }
    }
}
