use crate::input_handler::{operate as general_operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Value};

fn hex_decode(value: String) -> Result<Vec<u8>, ()> {
    let mut res = Vec::with_capacity(value.len() / 2);
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        let c = c.to_digit(16).ok_or(())?;
        let c2 = chars.next().and_then(|c| c.to_digit(16)).ok_or(())?;
        res.push((c * 16 + c2).try_into().map_err(|_| ())?);
    }
    Ok(res)
}

fn hex_digit(num: u8) -> char {
    match num {
        0..=9 => (num + b'0') as char,
        10..=15 => (num - 10 + b'A') as char,
        _ => unreachable!(),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut res = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        res.push(hex_digit(byte >> 4));
        res.push(hex_digit(byte & 0b1111));
    }
    res
}

#[derive(Clone)]
pub struct HexConfig {
    pub action_type: ActionType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    Encode,
    Decode,
}

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
    encoding_config: HexConfig,
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
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

    let args = Arguments {
        encoding_config: HexConfig { action_type },
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
    let hex_config = &args.encoding_config;

    match input {
        // Propagate existing errors.
        Value::Error { .. } => input.clone(),
        Value::Binary { val, .. } => match hex_config.action_type {
            ActionType::Encode => Value::string(hex_encode(val.as_ref()), command_span),
            ActionType::Decode => Value::Error {
                error: ShellError::UnsupportedInput(
                    "Binary data can only be encoded".to_string(),
                    "value originates from here".into(),
                    command_span,
                    // This line requires the Value::Error {} match above.
                    input.expect_span(),
                ),
            },
        },
        Value::String { val, .. } => {
            match hex_config.action_type {
                ActionType::Encode => Value::Error {
                    error: ShellError::UnsupportedInput(
                        "String value can only be decoded".to_string(),
                        "value originates from here".into(),
                        command_span,
                        // This line requires the Value::Error {} match above.
                        input.expect_span(),
                    ),
                },

                ActionType::Decode => {
                    // for decode, input val may contains invalid new line character, which is ok to omitted them by default.
                    let val = val.clone();
                    let val = val.replace("\r\n", "").replace('\n', "");

                    match hex_decode(val) {
                        Ok(decoded_value) => Value::binary(decoded_value, command_span),
                        Err(_) => Value::Error {
                            error: ShellError::GenericError(
                                "value could not be hex decoded".to_string(),
                                "invalid hex input".into(),
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
            error: ShellError::TypeMismatch {
                err_message: format!("string or binary, not {}", other.get_type()),
                span: other.span().unwrap_or(command_span),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{action, ActionType, Arguments, HexConfig};
    use nu_protocol::{Span, Value};

    #[test]
    fn hex_encode() {
        let word = Value::binary([77, 97, 110], Span::test_data());
        let expected = Value::test_string("4D616E");

        let actual = action(
            &word,
            &Arguments {
                encoding_config: HexConfig {
                    action_type: ActionType::Encode,
                },
                cell_paths: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn hex_decode() {
        let word = Value::test_string("4D61\r\n\n6E");
        let expected = Value::binary([77, 97, 110], Span::test_data());

        let actual = action(
            &word,
            &Arguments {
                encoding_config: HexConfig {
                    action_type: ActionType::Decode,
                },
                cell_paths: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }
}
