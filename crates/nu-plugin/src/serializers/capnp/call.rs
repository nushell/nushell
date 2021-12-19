use super::value;
use crate::{plugin_capnp::evaluated_call, EvaluatedCall};
use nu_protocol::{ShellError, Span, Spanned, Value};

pub(crate) fn serialize_call(
    call: &EvaluatedCall,
    mut builder: evaluated_call::Builder,
) -> Result<(), ShellError> {
    let mut head = builder.reborrow().init_head();
    head.set_start(call.head.start as u64);
    head.set_end(call.head.end as u64);

    serialize_positional(&call.positional, builder.reborrow());
    serialize_named(&call.named, builder)?;

    Ok(())
}

fn serialize_positional(positional: &[Value], mut builder: evaluated_call::Builder) {
    let mut positional_builder = builder.reborrow().init_positional(positional.len() as u32);

    for (index, value) in positional.iter().enumerate() {
        value::serialize_value(value, positional_builder.reborrow().get(index as u32))
    }
}

fn serialize_named(
    named: &[(Spanned<String>, Option<Value>)],
    mut builder: evaluated_call::Builder,
) -> Result<(), ShellError> {
    let mut named_builder = builder
        .reborrow()
        .init_named()
        .init_entries(named.len() as u32);

    for (index, (key, expression)) in named.iter().enumerate() {
        let mut entry_builder = named_builder.reborrow().get(index as u32);
        entry_builder
            .reborrow()
            .set_key(key.item.as_str())
            .map_err(|e| ShellError::PluginFailedToEncode(e.to_string()))?;

        if let Some(value) = expression {
            let value_builder = entry_builder.init_value();
            value::serialize_value(value, value_builder);
        }
    }

    Ok(())
}

pub(crate) fn deserialize_call(
    reader: evaluated_call::Reader,
) -> Result<EvaluatedCall, ShellError> {
    let head_reader = reader
        .get_head()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let head = Span {
        start: head_reader.get_start() as usize,
        end: head_reader.get_end() as usize,
    };

    let positional = deserialize_positionals(head, reader)?;
    let named = deserialize_named(head, reader)?;

    Ok(EvaluatedCall {
        head,
        positional,
        named,
    })
}

fn deserialize_positionals(
    span: Span,
    reader: evaluated_call::Reader,
) -> Result<Vec<Value>, ShellError> {
    let positional_reader = reader
        .get_positional()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    positional_reader
        .iter()
        .map(move |x| value::deserialize_value(x, span))
        .collect()
}

type NamedList = Vec<(Spanned<String>, Option<Value>)>;

fn deserialize_named(span: Span, reader: evaluated_call::Reader) -> Result<NamedList, ShellError> {
    let named_reader = reader
        .get_named()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let entries_list = named_reader
        .get_entries()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let mut entries: Vec<(Spanned<String>, Option<Value>)> =
        Vec::with_capacity(entries_list.len() as usize);

    for entry_reader in entries_list {
        let item = entry_reader
            .get_key()
            .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?
            .to_string();

        let value = if entry_reader.has_value() {
            let value_reader = entry_reader
                .get_value()
                .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let value = value::deserialize_value(value_reader, span)
                .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            Some(value)
        } else {
            None
        };

        let key = Spanned { item, span };

        entries.push((key, value))
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use capnp::serialize;
    use core::panic;

    use super::*;
    use nu_protocol::{Span, Spanned, Value};

    fn write_buffer(
        call: &EvaluatedCall,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        let mut message = ::capnp::message::Builder::new_default();

        let builder = message.init_root::<evaluated_call::Builder>();
        serialize_call(call, builder)?;

        serialize::write_message(writer, &message)
            .map_err(|e| ShellError::PluginFailedToLoad(e.to_string()))
    }

    fn read_buffer(reader: &mut impl std::io::BufRead) -> Result<EvaluatedCall, ShellError> {
        let message_reader =
            serialize::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

        let reader = message_reader
            .get_root::<evaluated_call::Reader>()
            .map_err(|e| ShellError::PluginFailedToLoad(e.to_string()))?;

        deserialize_call(reader)
    }

    #[test]
    fn call_round_trip() {
        let call = EvaluatedCall {
            head: Span { start: 0, end: 10 },
            positional: vec![
                Value::Float {
                    val: 1.0,
                    span: Span { start: 0, end: 10 },
                },
                Value::String {
                    val: "something".into(),
                    span: Span { start: 0, end: 10 },
                },
            ],
            named: vec![
                (
                    Spanned {
                        item: "name".to_string(),
                        span: Span { start: 0, end: 10 },
                    },
                    Some(Value::Float {
                        val: 1.0,
                        span: Span { start: 0, end: 10 },
                    }),
                ),
                (
                    Spanned {
                        item: "flag".to_string(),
                        span: Span { start: 0, end: 10 },
                    },
                    None,
                ),
            ],
        };

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&call, &mut buffer).expect("unable to serialize message");
        let returned_call = read_buffer(&mut buffer.as_slice()).expect("unable to read buffer");

        assert_eq!(call.head, returned_call.head);
        assert_eq!(call.positional.len(), returned_call.positional.len());

        call.positional
            .iter()
            .zip(returned_call.positional.iter())
            .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

        call.named
            .iter()
            .zip(returned_call.named.iter())
            .for_each(|(lhs, rhs)| {
                // Comparing the keys
                assert_eq!(lhs.0.item, rhs.0.item);

                match (&lhs.1, &rhs.1) {
                    (None, None) => {}
                    (Some(a), Some(b)) => assert_eq!(a, b),
                    _ => panic!("not matching values"),
                }
            });
    }
}
