use super::value;
use crate::{plugin_capnp::call_input, protocol::CallInput};
use nu_protocol::{ShellError, Span};

pub(crate) fn serialize_call_input(call_input: &CallInput, builder: call_input::Builder) {
    match call_input {
        CallInput::Value(value) => {
            value::serialize_value(value, builder.init_value());
        }
        CallInput::Data(_) => todo!(),
    };
}

pub(crate) fn deserialize_call_input(reader: call_input::Reader) -> Result<CallInput, ShellError> {
    match reader.which() {
        Err(capnp::NotInSchema(_)) => Err(ShellError::PluginFailedToDecode(
            "value not in schema".into(),
        )),
        Ok(call_input::Value(value_reader)) => {
            let value_reader =
                value_reader.map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let span_reader = value_reader
                .get_span()
                .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let span = Span {
                start: span_reader.get_start() as usize,
                end: span_reader.get_end() as usize,
            };

            Ok(CallInput::Value(value::deserialize_value(
                value_reader,
                span,
            )?))
        }
        Ok(call_input::PluginData(_)) => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::CallInput;
    use capnp::serialize;
    use nu_protocol::{Span, Value};

    pub fn write_buffer(
        call_input: &CallInput,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        let mut message = ::capnp::message::Builder::new_default();

        let mut builder = message.init_root::<call_input::Builder>();

        serialize_call_input(call_input, builder.reborrow());

        serialize::write_message(writer, &message)
            .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))
    }

    pub fn read_buffer(reader: &mut impl std::io::BufRead) -> Result<CallInput, ShellError> {
        let message_reader =
            serialize::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

        let reader = message_reader
            .get_root::<call_input::Reader>()
            .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

        deserialize_call_input(reader.reborrow())
    }

    #[test]
    fn callinput_value_round_trip() {
        let call_input = CallInput::Value(Value::String {
            val: "abc".to_string(),
            span: Span { start: 1, end: 20 },
        });

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&call_input, &mut buffer).expect("unable to serialize message");
        let returned_call_input =
            read_buffer(&mut buffer.as_slice()).expect("unable to deserialize message");

        assert_eq!(call_input, returned_call_input)
    }
}
