use nu_cmd_base::input_handler::{operate as general_operate, CmdArgument};
use nu_engine::command_prelude::*;

enum HexDecodingError {
    InvalidLength(usize),
    InvalidDigit(usize, char),
}

fn hex_decode(value: &str) -> Result<Vec<u8>, HexDecodingError> {
    let mut digits = value
        .chars()
        .enumerate()
        .filter(|(_, c)| !c.is_whitespace());

    let mut res = Vec::with_capacity(value.len() / 2);
    loop {
        let c1 = match digits.next() {
            Some((ind, c)) => match c.to_digit(16) {
                Some(d) => d,
                None => return Err(HexDecodingError::InvalidDigit(ind, c)),
            },
            None => return Ok(res),
        };
        let c2 = match digits.next() {
            Some((ind, c)) => match c.to_digit(16) {
                Some(d) => d,
                None => return Err(HexDecodingError::InvalidDigit(ind, c)),
            },
            None => {
                return Err(HexDecodingError::InvalidLength(value.len()));
            }
        };
        res.push((c1 << 4 | c2) as u8);
    }
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
            ActionType::Decode => Value::error(
                ShellError::UnsupportedInput { msg: "Binary data can only be encoded".to_string(), input: "value originates from here".into(), msg_span: command_span, input_span: input.span() },
                command_span,
            ),
        },
        Value::String { val, .. } => {
            match hex_config.action_type {
                ActionType::Encode => Value::error(
                    ShellError::UnsupportedInput { msg: "String value can only be decoded".to_string(), input: "value originates from here".into(), msg_span: command_span, input_span: input.span() },
                    command_span,
                ),

                ActionType::Decode => match hex_decode(val.as_ref()) {
                    Ok(decoded_value) => Value::binary(decoded_value, command_span),
                    Err(HexDecodingError::InvalidLength(len)) => Value::error(ShellError::GenericError {
                            error: "value could not be hex decoded".into(),
                            msg: format!("invalid hex input length: {len}. The length should be even"),
                            span: Some(command_span),
                            help: None,
                            inner: vec![],
                        },
                        command_span,
                    ),
                    Err(HexDecodingError::InvalidDigit(index, digit)) => Value::error(ShellError::GenericError {
                            error: "value could not be hex decoded".into(),
                            msg: format!("invalid hex digit: '{digit}' at index {index}. Only 0-9, A-F, a-f are allowed in hex encoding"),
                            span: Some(command_span),
                            help: None,
                            inner: vec![],
                        },
                        command_span,
                    ),
                },
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
        let word = Value::test_string("4D 61\r\n\n6E");
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
