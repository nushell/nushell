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

pub(crate) fn deserialize_call_input(
    reader: call_input::Reader,
    head: Span,
) -> Result<CallInput, ShellError> {
    match reader.which() {
        Err(capnp::NotInSchema(_)) => Err(ShellError::PluginFailedToDecode(
            "value not in schema".into(),
        )),
        Ok(call_input::Value(value_reader)) => {
            let value_reader =
                value_reader.map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            Ok(CallInput::Value(value::deserialize_value(
                value_reader,
                head,
            )?))
        }
        Ok(call_input::PluginData(_)) => todo!(),
    }
}
