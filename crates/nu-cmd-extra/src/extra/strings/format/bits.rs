use std::io::{self, Read, Write};

use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

use nu_protocol::{Signals, shell_error::io::IoError};
use num_traits::ToPrimitive;

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct FormatBits;

impl Command for FormatBits {
    fn name(&self) -> &str {
        "format bits"
    }

    fn signature(&self) -> Signature {
        Signature::build("format bits")
            .input_output_types(vec![
                (Type::Binary, Type::String),
                (Type::Int, Type::String),
                (Type::Filesize, Type::String),
                (Type::Duration, Type::String),
                (Type::String, Type::String),
                (Type::Bool, Type::String),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true) // TODO: supply exhaustive examples
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert value to a string of binary data represented by 0 and 1."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "cast", "binary"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        format_bits(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert a binary value into a string, padded to 8 places with 0s",
                example: "0x[1] | format bits",
                result: Some(Value::string("00000001", Span::test_data())),
            },
            Example {
                description: "convert an int into a string, padded to 8 places with 0s",
                example: "1 | format bits",
                result: Some(Value::string("00000001", Span::test_data())),
            },
            Example {
                description: "convert a filesize value into a string, padded to 8 places with 0s",
                example: "1b | format bits",
                result: Some(Value::string("00000001", Span::test_data())),
            },
            Example {
                description: "convert a duration value into a string, padded to 8 places with 0s",
                example: "1ns | format bits",
                result: Some(Value::string("00000001", Span::test_data())),
            },
            Example {
                description: "convert a boolean value into a string, padded to 8 places with 0s",
                example: "true | format bits",
                result: Some(Value::string("00000001", Span::test_data())),
            },
            Example {
                description: "convert a string into a raw binary string, padded with 0s to 8 places",
                example: "'nushell.sh' | format bits",
                result: Some(Value::string(
                    "01101110 01110101 01110011 01101000 01100101 01101100 01101100 00101110 01110011 01101000",
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn format_bits(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let cell_paths = call.rest(engine_state, stack, 0)?;
    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

    if let PipelineData::ByteStream(stream, metadata) = input {
        Ok(PipelineData::byte_stream(
            byte_stream_to_bits(stream, head),
            metadata,
        ))
    } else {
        let args = Arguments { cell_paths };
        operate(action, args, input, call.head, engine_state.signals())
    }
}

fn byte_stream_to_bits(stream: ByteStream, head: Span) -> ByteStream {
    if let Some(mut reader) = stream.reader() {
        let mut is_first = true;
        ByteStream::from_fn(
            head,
            Signals::empty(),
            ByteStreamType::String,
            move |buffer| {
                let mut byte = [0];
                if reader
                    .read(&mut byte[..])
                    .map_err(|err| IoError::new(err, head, None))?
                    > 0
                {
                    // Format the byte as bits
                    if is_first {
                        is_first = false;
                    } else {
                        buffer.push(b' ');
                    }
                    write!(buffer, "{:08b}", byte[0]).expect("format failed");
                    Ok(true)
                } else {
                    // EOF
                    Ok(false)
                }
            },
        )
    } else {
        ByteStream::read(io::empty(), head, Signals::empty(), ByteStreamType::String)
    }
}

fn convert_to_smallest_number_type(num: i64, span: Span) -> Value {
    if let Some(v) = num.to_i8() {
        let bytes = v.to_ne_bytes();
        let mut raw_string = "".to_string();
        for ch in bytes {
            raw_string.push_str(&format!("{ch:08b} "));
        }
        Value::string(raw_string.trim(), span)
    } else if let Some(v) = num.to_i16() {
        let bytes = v.to_ne_bytes();
        let mut raw_string = "".to_string();
        for ch in bytes {
            raw_string.push_str(&format!("{ch:08b} "));
        }
        Value::string(raw_string.trim(), span)
    } else if let Some(v) = num.to_i32() {
        let bytes = v.to_ne_bytes();
        let mut raw_string = "".to_string();
        for ch in bytes {
            raw_string.push_str(&format!("{ch:08b} "));
        }
        Value::string(raw_string.trim(), span)
    } else {
        let bytes = num.to_ne_bytes();
        let mut raw_string = "".to_string();
        for ch in bytes {
            raw_string.push_str(&format!("{ch:08b} "));
        }
        Value::string(raw_string.trim(), span)
    }
}

fn action(input: &Value, _args: &Arguments, span: Span) -> Value {
    match input {
        Value::Binary { val, .. } => {
            let mut raw_string = "".to_string();
            for ch in val {
                raw_string.push_str(&format!("{ch:08b} "));
            }
            Value::string(raw_string.trim(), span)
        }
        Value::Int { val, .. } => convert_to_smallest_number_type(*val, span),
        Value::Filesize { val, .. } => convert_to_smallest_number_type(val.get(), span),
        Value::Duration { val, .. } => convert_to_smallest_number_type(*val, span),
        Value::String { val, .. } => {
            let raw_bytes = val.as_bytes();
            let mut raw_string = "".to_string();
            for ch in raw_bytes {
                raw_string.push_str(&format!("{ch:08b} "));
            }
            Value::string(raw_string.trim(), span)
        }
        Value::Bool { val, .. } => {
            let v = <i64 as From<bool>>::from(*val);
            convert_to_smallest_number_type(v, span)
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int, filesize, string, duration, binary, or bool".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FormatBits {})
    }
}
