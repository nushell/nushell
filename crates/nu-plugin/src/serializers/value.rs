use crate::plugin::PluginError;
use crate::plugin_capnp::value;
use nu_protocol::{Span, Value};

pub(crate) fn serialize_value(value: &Value, mut builder: value::Builder) {
    let value_span = match value {
        Value::Nothing { span } => {
            builder.set_void(());
            *span
        }
        Value::Bool { val, span } => {
            builder.set_bool(*val);
            *span
        }
        Value::Int { val, span } => {
            builder.set_int(*val);
            *span
        }
        Value::Float { val, span } => {
            builder.set_float(*val);
            *span
        }
        Value::String { val, span } => {
            builder.set_string(val);
            *span
        }
        Value::List { vals, span } => {
            let mut list_builder = builder.reborrow().init_list(vals.len() as u32);
            for (index, value) in vals.iter().enumerate() {
                let inner_builder = list_builder.reborrow().get(index as u32);
                serialize_value(value, inner_builder);
            }

            *span
        }
        _ => {
            // If there is the need to pass other type of value to the plugin
            // we have to define the encoding for that object in this match
            Span::unknown()
        }
    };

    let mut span = builder.reborrow().init_span();
    span.set_start(value_span.start as u64);
    span.set_end(value_span.end as u64);
}

pub(crate) fn deserialize_value(reader: value::Reader) -> Result<Value, PluginError> {
    let span_reader = reader
        .get_span()
        .map_err(|e| PluginError::DecodingError(e.to_string()))?;

    let span = Span {
        start: span_reader.get_start() as usize,
        end: span_reader.get_end() as usize,
    };

    match reader.which() {
        Ok(value::Void(())) => Ok(Value::Nothing { span }),
        Ok(value::Bool(val)) => Ok(Value::Bool { val, span }),
        Ok(value::Int(val)) => Ok(Value::Int { val, span }),
        Ok(value::Float(val)) => Ok(Value::Float { val, span }),
        Ok(value::String(val)) => {
            let string = val
                .map_err(|e| PluginError::DecodingError(e.to_string()))?
                .to_string();
            Ok(Value::String { val: string, span })
        }
        Ok(value::List(vals)) => {
            let values = vals.map_err(|e| PluginError::DecodingError(e.to_string()))?;

            let values_list = values
                .iter()
                .map(deserialize_value)
                .collect::<Result<Vec<Value>, PluginError>>()?;

            Ok(Value::List {
                vals: values_list,
                span,
            })
        }
        Err(capnp::NotInSchema(_)) => Ok(Value::Nothing {
            span: Span::unknown(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use capnp::serialize_packed;
    use nu_protocol::{Span, Value};

    pub fn write_buffer(
        value: &Value,
        writer: &mut impl std::io::Write,
    ) -> Result<(), PluginError> {
        let mut message = ::capnp::message::Builder::new_default();

        let mut builder = message.init_root::<value::Builder>();

        serialize_value(value, builder.reborrow());

        serialize_packed::write_message(writer, &message)
            .map_err(|e| PluginError::EncodingError(e.to_string()))
    }

    pub fn read_buffer(reader: &mut impl std::io::BufRead) -> Result<Value, PluginError> {
        let message_reader =
            serialize_packed::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

        let reader = message_reader
            .get_root::<value::Reader>()
            .map_err(|e| PluginError::DecodingError(e.to_string()))?;

        deserialize_value(reader.reborrow())
    }

    #[test]
    fn value_round_trip() {
        let values = [
            Value::Bool {
                val: false,
                span: Span { start: 1, end: 20 },
            },
            Value::Int {
                val: 10,
                span: Span { start: 2, end: 30 },
            },
            Value::Float {
                val: 10.0,
                span: Span { start: 3, end: 40 },
            },
            Value::String {
                val: "a string".into(),
                span: Span { start: 4, end: 50 },
            },
        ];

        for value in values {
            let mut buffer: Vec<u8> = Vec::new();
            write_buffer(&value, &mut buffer).expect("unable to serialize message");
            let returned_value =
                read_buffer(&mut buffer.as_slice()).expect("unable to deserialize message");

            assert_eq!(value, returned_value)
        }
    }

    #[test]
    fn value_nothing_round_trip() {
        // Since nothing doesn't implement PartialOrd, we only compare that the
        // encoded and decoded spans are correct
        let value = Value::Nothing {
            span: Span { start: 0, end: 10 },
        };

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&value, &mut buffer).expect("unable to serialize message");
        let returned_value =
            read_buffer(&mut buffer.as_slice()).expect("unable to deserialize message");

        assert_eq!(
            value.span().expect("span"),
            returned_value.span().expect("span")
        )
    }

    #[test]
    fn list_round_trip() {
        let values = vec![
            Value::Bool {
                val: false,
                span: Span { start: 1, end: 20 },
            },
            Value::Int {
                val: 10,
                span: Span { start: 2, end: 30 },
            },
            Value::Float {
                val: 10.0,
                span: Span { start: 3, end: 40 },
            },
            Value::String {
                val: "a string".into(),
                span: Span { start: 4, end: 50 },
            },
        ];

        let value = Value::List {
            vals: values,
            span: Span { start: 1, end: 10 },
        };

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&value, &mut buffer).expect("unable to serialize message");
        let returned_value =
            read_buffer(&mut buffer.as_slice()).expect("unable to deserialize message");

        assert_eq!(
            value.span().expect("span"),
            returned_value.span().expect("span")
        )
    }

    #[test]
    fn nested_list_round_trip() {
        let inner_values = vec![
            Value::Bool {
                val: false,
                span: Span { start: 1, end: 20 },
            },
            Value::Int {
                val: 10,
                span: Span { start: 2, end: 30 },
            },
            Value::Float {
                val: 10.0,
                span: Span { start: 3, end: 40 },
            },
            Value::String {
                val: "inner string".into(),
                span: Span { start: 4, end: 50 },
            },
        ];

        let values = vec![
            Value::Bool {
                val: true,
                span: Span { start: 1, end: 20 },
            },
            Value::Int {
                val: 66,
                span: Span { start: 2, end: 30 },
            },
            Value::Float {
                val: 66.6,
                span: Span { start: 3, end: 40 },
            },
            Value::String {
                val: "a string".into(),
                span: Span { start: 4, end: 50 },
            },
            Value::List {
                vals: inner_values,
                span: Span { start: 5, end: 60 },
            },
        ];

        let value = Value::List {
            vals: values,
            span: Span { start: 1, end: 10 },
        };

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&value, &mut buffer).expect("unable to serialize message");
        let returned_value =
            read_buffer(&mut buffer.as_slice()).expect("unable to deserialize message");

        assert_eq!(
            value.span().expect("span"),
            returned_value.span().expect("span")
        )
    }
}
