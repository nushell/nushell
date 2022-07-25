use crate::{plugin_capnp::plugin_data, protocol::PluginData};
use nu_protocol::{ShellError, Span};

pub(crate) fn serialize_plugin_data(plugin_data: &PluginData, mut builder: plugin_data::Builder) {
    builder.set_data(&plugin_data.data);

    let mut span_builder = builder.init_span();
    span_builder.set_start(plugin_data.span.start as u64);
    span_builder.set_end(plugin_data.span.end as u64);
}

pub(crate) fn deserialize_plugin_data(
    reader: plugin_data::Reader,
) -> Result<PluginData, ShellError> {
    let data = reader
        .get_data()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let span_reader = reader
        .get_span()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let span = Span {
        start: span_reader.get_start() as usize,
        end: span_reader.get_end() as usize,
    };

    Ok(PluginData {
        data: data.to_vec(),
        span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use capnp::serialize;
    use nu_protocol::Span;

    pub fn write_buffer(
        plugin_data: &PluginData,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        let mut message = ::capnp::message::Builder::new_default();

        let mut builder = message.init_root::<plugin_data::Builder>();

        serialize_plugin_data(plugin_data, builder.reborrow());

        serialize::write_message(writer, &message)
            .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))
    }

    pub fn read_buffer(reader: &mut impl std::io::BufRead) -> Result<PluginData, ShellError> {
        let message_reader =
            serialize::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

        let reader = message_reader
            .get_root::<plugin_data::Reader>()
            .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

        deserialize_plugin_data(reader.reborrow())
    }

    #[test]
    fn plugin_data_round_trip() {
        let plugin_data = PluginData {
            data: vec![1, 2, 3, 4, 5, 6, 7],
            span: Span { start: 1, end: 20 },
        };

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&plugin_data, &mut buffer).expect("unable to serialize message");
        let returned_plugin_data =
            read_buffer(&mut buffer.as_slice()).expect("unable to deserialize message");

        assert_eq!(plugin_data, returned_plugin_data)
    }
}
