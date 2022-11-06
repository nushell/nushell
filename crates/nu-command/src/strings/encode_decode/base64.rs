use crate::input_handler::{operate as general_operate, CmdArgument};
use base64::{decode_config, encode_config};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Spanned, Value};

pub const CHARACTER_SET_DESC: &str = "specify the character rules for encoding the input.\n\
                    \tValid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding',\
                    'binhex', 'bcrypt', 'crypt'";

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
    let binary = call.has_flag("binary");
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
    let base64_config_enum: base64::Config = match config_character_set.item.as_str() {
        "standard" => base64::STANDARD,
        "standard-no-padding" => base64::STANDARD_NO_PAD,
        "url-safe" => base64::URL_SAFE,
        "url-safe-no-padding" => base64::URL_SAFE_NO_PAD,
        "binhex" => base64::BINHEX,
        "bcrypt" => base64::BCRYPT,
        "crypt" => base64::CRYPT,
        not_valid => return Value::Error { error:ShellError::GenericError(
            "value is not an accepted character set".to_string(),
            format!(
                "{} is not a valid character-set.\nPlease use `help hash base64` to see a list of valid character sets.",
                not_valid
            ),
            Some(config_character_set.span),
            None,
            Vec::new(),
        )}
    };
    match input {
        Value::Binary { val, .. } => match base64_config.action_type {
            ActionType::Encode => {
                Value::string(encode_config(&val, base64_config_enum), command_span)
            }
            ActionType::Decode => Value::Error {
                error: ShellError::UnsupportedInput(
                    "Binary data can only support encoding".to_string(),
                    command_span,
                ),
            },
        },
        Value::String {
            val,
            span: value_span,
        } => {
            match base64_config.action_type {
                ActionType::Encode => {
                    Value::string(encode_config(val, base64_config_enum), command_span)
                }

                ActionType::Decode => {
                    // for decode, input val may contains invalid new line character, which is ok to omitted them by default.
                    let val = val.clone();
                    let val = val.replace("\r\n", "").replace('\n', "");

                    match decode_config(&val, base64_config_enum) {
                        Ok(decoded_value) => {
                            if output_binary {
                                Value::binary(decoded_value, command_span)
                            } else {
                                match String::from_utf8(decoded_value) {
                                    Ok(string_value) => Value::string(string_value, command_span),
                                    Err(e) => Value::Error {
                                        error: ShellError::GenericError(
                                            "base64 payload isn't a valid utf-8 sequence"
                                                .to_owned(),
                                            e.to_string(),
                                            Some(*value_span),
                                            Some("consider using the `--binary` flag".to_owned()),
                                            Vec::new(),
                                        ),
                                    },
                                }
                            }
                        }
                        Err(_) => Value::Error {
                            error: ShellError::GenericError(
                                "value could not be base64 decoded".to_string(),
                                format!(
                                    "invalid base64 input for character set {}",
                                    &config_character_set.item
                                ),
                                Some(command_span),
                                None,
                                Vec::new(),
                            ),
                        },
                    }
                }
            }
        }
        other => Value::Error {
            error: ShellError::TypeMismatch(
                format!("value is {}, not string", other.get_type()),
                other.span().unwrap_or(command_span),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{action, ActionType, Arguments, Base64Config};
    use nu_protocol::{Span, Spanned, Value};

    #[test]
    fn base64_encode_standard() {
        let word = Value::string("Some Data Padding", Span::test_data());
        let expected = Value::string("U29tZSBEYXRhIFBhZGRpbmc=", Span::test_data());

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
        let word = Value::string("Some Data Padding", Span::test_data());
        let expected = Value::string("U29tZSBEYXRhIFBhZGRpbmc", Span::test_data());

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
        let word = Value::string("this is for url", Span::test_data());
        let expected = Value::string("dGhpcyBpcyBmb3IgdXJs", Span::test_data());

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
        let word = Value::string("A5\"KC9jRB@IIF'8bF!", Span::test_data());
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
        let word = Value::string("A5\"KC9jRB\n@IIF'8bF!", Span::test_data());
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
        let word = Value::Binary {
            val: vec![77, 97, 110],
            span: Span::test_data(),
        };
        let expected = Value::string("TWFu", Span::test_data());

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
        let word = Value::Binary {
            val: vec![77, 97, 110],
            span: Span::test_data(),
        };

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
