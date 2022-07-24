use super::signature::deserialize_signature;
use super::{call, call_input, plugin_data, signature, value};
use crate::plugin_capnp::{plugin_call, plugin_response};
use crate::protocol::{CallInfo, LabeledError, PluginCall, PluginResponse};
use capnp::serialize;
use nu_protocol::{ShellError, Signature, Span};

pub fn encode_call(
    plugin_call: &PluginCall,
    writer: &mut impl std::io::Write,
) -> Result<(), ShellError> {
    let mut message = ::capnp::message::Builder::new_default();

    let mut builder = message.init_root::<plugin_call::Builder>();

    match &plugin_call {
        PluginCall::Signature => builder.set_signature(()),
        PluginCall::CallInfo(call_info) => {
            let mut call_info_builder = builder.reborrow().init_call_info();

            // Serializing name from the call
            call_info_builder.set_name(call_info.name.as_str());

            // Serializing argument information from the call
            let call_builder = call_info_builder
                .reborrow()
                .get_call()
                .map_err(|e| ShellError::PluginFailedToEncode(e.to_string()))?;

            call::serialize_call(&call_info.call, call_builder)
                .map_err(|e| ShellError::PluginFailedToEncode(e.to_string()))?;

            // Serializing the input value from the call info
            let call_input_builder = call_info_builder
                .reborrow()
                .get_input()
                .map_err(|e| ShellError::PluginFailedToEncode(e.to_string()))?;

            call_input::serialize_call_input(&call_info.input, call_input_builder);
        }
        PluginCall::CollapseCustomValue(plugin_data) => {
            let builder = builder.init_collapse_custom_value();

            plugin_data::serialize_plugin_data(plugin_data, builder);
        }
    };

    serialize::write_message(writer, &message)
        .map_err(|e| ShellError::PluginFailedToEncode(e.to_string()))
}

pub fn decode_call(reader: &mut impl std::io::BufRead) -> Result<PluginCall, ShellError> {
    let message_reader = serialize::read_message(reader, ::capnp::message::ReaderOptions::new())
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let reader = message_reader
        .get_root::<plugin_call::Reader>()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    match reader.which() {
        Err(capnp::NotInSchema(_)) => Err(ShellError::PluginFailedToDecode(
            "value not in schema".into(),
        )),
        Ok(plugin_call::Signature(())) => Ok(PluginCall::Signature),
        Ok(plugin_call::CallInfo(reader)) => {
            let reader = reader.map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let name = reader
                .get_name()
                .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let call_reader = reader
                .get_call()
                .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let call = call::deserialize_call(call_reader)?;

            let input_reader = reader
                .get_input()
                .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let input = call_input::deserialize_call_input(input_reader)?;

            Ok(PluginCall::CallInfo(CallInfo {
                name: name.to_string(),
                call,
                input,
            }))
        }
        Ok(plugin_call::CollapseCustomValue(reader)) => {
            let reader = reader.map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let plugin_data = plugin_data::deserialize_plugin_data(reader)?;

            Ok(PluginCall::CollapseCustomValue(plugin_data))
        }
    }
}

pub fn encode_response(
    plugin_response: &PluginResponse,
    writer: &mut impl std::io::Write,
) -> Result<(), ShellError> {
    let mut message = ::capnp::message::Builder::new_default();

    let mut builder = message.init_root::<plugin_response::Builder>();

    match &plugin_response {
        PluginResponse::Error(msg) => {
            let mut error_builder = builder.reborrow().init_error();
            error_builder.set_label(&msg.label);
            error_builder.set_msg(&msg.msg);

            if let Some(span) = msg.span {
                let mut span_builder = error_builder.reborrow().init_span();
                span_builder.set_start(span.start as u64);
                span_builder.set_end(span.end as u64);
            }
        }
        PluginResponse::Signature(signatures) => {
            let mut signature_list_builder =
                builder.reborrow().init_signature(signatures.len() as u32);

            for (index, signature) in signatures.iter().enumerate() {
                let signature_builder = signature_list_builder.reborrow().get(index as u32);
                signature::serialize_signature(signature, signature_builder)
            }
        }
        PluginResponse::Value(val) => {
            let value_builder = builder.reborrow().init_value();
            value::serialize_value(val, value_builder);
        }
        PluginResponse::PluginData(..) => todo!(),
    };

    serialize::write_message(writer, &message)
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))
}

pub fn decode_response(reader: &mut impl std::io::BufRead) -> Result<PluginResponse, ShellError> {
    let message_reader = serialize::read_message(reader, ::capnp::message::ReaderOptions::new())
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let reader = message_reader
        .get_root::<plugin_response::Reader>()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    match reader.which() {
        Err(capnp::NotInSchema(_)) => Err(ShellError::PluginFailedToDecode(
            "value not in schema".into(),
        )),
        Ok(plugin_response::Error(reader)) => {
            let reader = reader.map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let msg = reader
                .get_msg()
                .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let label = reader
                .get_label()
                .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let span = if reader.has_span() {
                let span = reader
                    .get_span()
                    .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

                Some(Span {
                    start: span.get_start() as usize,
                    end: span.get_end() as usize,
                })
            } else {
                None
            };

            let error = LabeledError {
                label: label.into(),
                msg: msg.into(),
                span,
            };

            Ok(PluginResponse::Error(error))
        }
        Ok(plugin_response::Signature(reader)) => {
            let reader = reader.map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let signatures = reader
                .iter()
                .map(deserialize_signature)
                .collect::<Result<Vec<Signature>, ShellError>>()?;

            Ok(PluginResponse::Signature(signatures))
        }
        Ok(plugin_response::Value(reader)) => {
            let reader = reader.map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let span = reader
                .get_span()
                .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            let val = value::deserialize_value(
                reader,
                Span {
                    start: span.get_start() as usize,
                    end: span.get_end() as usize,
                },
            )
            .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

            Ok(PluginResponse::Value(Box::new(val)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        CallInput, EvaluatedCall, LabeledError, PluginCall, PluginData, PluginResponse,
    };
    use nu_protocol::{Signature, Span, Spanned, SyntaxShape, Value};

    #[test]
    fn callinfo_round_trip_signature() {
        let plugin_call = PluginCall::Signature;

        let mut buffer: Vec<u8> = Vec::new();
        encode_call(&plugin_call, &mut buffer).expect("unable to serialize message");
        let returned = decode_call(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginCall::Signature => {}
            PluginCall::CallInfo(_) => panic!("decoded into wrong value"),
            PluginCall::CollapseCustomValue(_) => panic!("decoded into wrong value"),
        }
    }

    #[test]
    fn callinfo_round_trip_callinfo() {
        let name = "test".to_string();

        let input = CallInput::Value(Value::Bool {
            val: false,
            span: Span { start: 1, end: 20 },
        });

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
            named: vec![(
                Spanned {
                    item: "name".to_string(),
                    span: Span { start: 0, end: 10 },
                },
                Some(Value::Float {
                    val: 1.0,
                    span: Span { start: 0, end: 10 },
                }),
            )],
        };

        let plugin_call = PluginCall::CallInfo(CallInfo {
            name: name.clone(),
            call: call.clone(),
            // Avoiding having to implement Clone on CallInput just for tests
            input: match &input {
                CallInput::Value(value) => CallInput::Value(value.clone()),
                CallInput::Data(plugin_data) => CallInput::Data(PluginData {
                    data: plugin_data.data.clone(),
                    span: plugin_data.span,
                }),
            },
        });

        let mut buffer: Vec<u8> = Vec::new();
        encode_call(&plugin_call, &mut buffer).expect("unable to serialize message");
        let returned = decode_call(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginCall::Signature => panic!("returned wrong call type"),
            PluginCall::CallInfo(call_info) => {
                assert_eq!(name, call_info.name);
                assert_eq!(&input, &call_info.input);
                assert_eq!(call.head, call_info.call.head);
                assert_eq!(call.positional.len(), call_info.call.positional.len());

                call.positional
                    .iter()
                    .zip(call_info.call.positional.iter())
                    .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                call.named
                    .iter()
                    .zip(call_info.call.named.iter())
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
            PluginCall::CollapseCustomValue(_) => panic!("returned wrong call type"),
        }
    }

    #[test]
    fn callinfo_round_trip_collapsecustomvalue() {
        let data = vec![1, 2, 3, 4, 5, 6, 7];
        let span = Span { start: 0, end: 20 };

        let collapse_custom_value = PluginCall::CollapseCustomValue(PluginData {
            data: data.clone(),
            span,
        });

        let mut buffer: Vec<u8> = Vec::new();
        encode_call(&collapse_custom_value, &mut buffer).expect("unable to serialize message");
        let returned = decode_call(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginCall::Signature => panic!("returned wrong call type"),
            PluginCall::CallInfo(_) => panic!("returned wrong call type"),
            PluginCall::CollapseCustomValue(plugin_data) => {
                assert_eq!(data, plugin_data.data);
                assert_eq!(span, plugin_data.span);
            }
        }
    }

    #[test]
    fn response_round_trip_signature() {
        let signature = Signature::build("nu-plugin")
            .required("first", SyntaxShape::String, "first required")
            .required("second", SyntaxShape::Int, "second required")
            .required_named("first-named", SyntaxShape::String, "first named", Some('f'))
            .required_named(
                "second-named",
                SyntaxShape::String,
                "second named",
                Some('s'),
            )
            .rest("remaining", SyntaxShape::Int, "remaining");

        let response = PluginResponse::Signature(vec![signature.clone()]);

        let mut buffer: Vec<u8> = Vec::new();
        encode_response(&response, &mut buffer).expect("unable to serialize message");
        let returned =
            decode_response(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginResponse::Error(_) => panic!("returned wrong call type"),
            PluginResponse::Value(_) => panic!("returned wrong call type"),
            PluginResponse::PluginData(..) => panic!("returned wrong call type"),
            PluginResponse::Signature(returned_signature) => {
                assert!(returned_signature.len() == 1);
                assert_eq!(signature.name, returned_signature[0].name);
                assert_eq!(signature.usage, returned_signature[0].usage);
                assert_eq!(signature.extra_usage, returned_signature[0].extra_usage);
                assert_eq!(signature.is_filter, returned_signature[0].is_filter);

                signature
                    .required_positional
                    .iter()
                    .zip(returned_signature[0].required_positional.iter())
                    .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                signature
                    .optional_positional
                    .iter()
                    .zip(returned_signature[0].optional_positional.iter())
                    .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                signature
                    .named
                    .iter()
                    .zip(returned_signature[0].named.iter())
                    .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                assert_eq!(
                    signature.rest_positional,
                    returned_signature[0].rest_positional,
                );
            }
        }
    }

    #[test]
    fn response_round_trip_value() {
        let value = Value::Int {
            val: 10,
            span: Span { start: 2, end: 30 },
        };

        let response = PluginResponse::Value(Box::new(value.clone()));

        let mut buffer: Vec<u8> = Vec::new();
        encode_response(&response, &mut buffer).expect("unable to serialize message");
        let returned =
            decode_response(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginResponse::Error(_) => panic!("returned wrong call type"),
            PluginResponse::Signature(_) => panic!("returned wrong call type"),
            PluginResponse::PluginData(..) => panic!("returned wrong call type"),
            PluginResponse::Value(returned_value) => {
                assert_eq!(&value, returned_value.as_ref())
            }
        }
    }

    #[test]
    fn response_round_trip_error() {
        let error = LabeledError {
            label: "label".into(),
            msg: "msg".into(),
            span: Some(Span { start: 2, end: 30 }),
        };
        let response = PluginResponse::Error(error.clone());

        let mut buffer: Vec<u8> = Vec::new();
        encode_response(&response, &mut buffer).expect("unable to serialize message");
        let returned =
            decode_response(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginResponse::Error(msg) => assert_eq!(error, msg),
            PluginResponse::Signature(_) => panic!("returned wrong call type"),
            PluginResponse::Value(_) => panic!("returned wrong call type"),
            PluginResponse::PluginData(..) => panic!("returned wrong call type"),
        }
    }

    #[test]
    fn response_round_trip_error_none() {
        let error = LabeledError {
            label: "label".into(),
            msg: "msg".into(),
            span: None,
        };
        let response = PluginResponse::Error(error.clone());

        let mut buffer: Vec<u8> = Vec::new();
        encode_response(&response, &mut buffer).expect("unable to serialize message");
        let returned =
            decode_response(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginResponse::Error(msg) => assert_eq!(error, msg),
            PluginResponse::Signature(_) => panic!("returned wrong call type"),
            PluginResponse::Value(_) => panic!("returned wrong call type"),
            PluginResponse::PluginData(..) => panic!("returned wrong call type"),
        }
    }
}
