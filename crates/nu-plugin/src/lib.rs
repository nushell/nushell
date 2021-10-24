pub mod value_capnp {
    include!(concat!(env!("OUT_DIR"), "/value_capnp.rs"));
}

pub mod plugin_value {

    use crate::value_capnp::value;
    use capnp::serialize_packed;
    use nu_protocol::{Span, Value};

    pub fn serialize_message(value: &Value, writer: &mut impl std::io::Write) -> capnp::Result<()> {
        let mut message = ::capnp::message::Builder::new_default();

        let mut builder = message.init_root::<value::Builder>();

        let value_span = serialize_value(value, builder.reborrow());
        let mut span = builder.reborrow().init_span();
        span.set_start(value_span.start as u64);
        span.set_end(value_span.end as u64);

        serialize_packed::write_message(writer, &message)
    }

    fn serialize_value(value: &Value, mut builder: value::Builder) -> Span {
        match value {
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
                builder.set_string(&val);
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
            _ => Span::unknown(),
        }
    }

    pub fn deserialize_message(reader: &mut impl std::io::BufRead) -> Value {
        let message_reader =
            serialize_packed::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

        let plugin_value = message_reader.get_root::<value::Reader>().unwrap();

        let span_reader = plugin_value.get_span().unwrap();
        let span = Span {
            start: span_reader.get_start() as usize,
            end: span_reader.get_end() as usize,
        };

        deserialize_value(span, plugin_value.reborrow())
    }

    fn deserialize_value(span: Span, reader: value::Reader) -> Value {
        match reader.which() {
            Ok(value::Void(())) => Value::Nothing { span },
            Ok(value::Bool(val)) => Value::Bool { val, span },
            Ok(value::Int(val)) => Value::Int { val, span },
            Ok(value::Float(val)) => Value::Float { val, span },
            Ok(value::String(val)) => Value::String {
                val: val.unwrap().to_string(),
                span,
            },
            Ok(value::List(vals)) => {
                let values = vals.expect("something");

                let values_list = values
                    .iter()
                    .map(|value| match value.which() {
                        Ok(value::Void(())) => Value::Nothing { span },
                        Ok(value::Bool(val)) => Value::Bool { val, span },
                        Ok(value::Int(val)) => Value::Int { val, span },
                        Ok(value::Float(val)) => Value::Float { val, span },
                        Ok(value::String(val)) => Value::String {
                            val: val.unwrap().to_string(),
                            span,
                        },
                        Ok(value::List(_)) => Value::Nothing { span },
                        Err(capnp::NotInSchema(_)) => Value::Nothing {
                            span: Span::unknown(),
                        },
                    })
                    .collect::<Vec<Value>>();

                Value::List {
                    vals: values_list,
                    span,
                }
            }
            Err(capnp::NotInSchema(_)) => Value::Nothing {
                span: Span::unknown(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{Span, Value};

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
            plugin_value::serialize_message(&value, &mut buffer).expect("unable to write message");
            let returned_value = plugin_value::deserialize_message(&mut buffer.as_slice());

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
        plugin_value::serialize_message(&value, &mut buffer).expect("unable to write message");
        let returned_value = plugin_value::deserialize_message(&mut buffer.as_slice());

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
        plugin_value::serialize_message(&value, &mut buffer).expect("unable to write message");
        let returned_value = plugin_value::deserialize_message(&mut buffer.as_slice());

        assert_eq!(
            value.span().expect("span"),
            returned_value.span().expect("span")
        )
    }
}
