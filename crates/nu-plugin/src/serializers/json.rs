use nu_protocol::ShellError;

use crate::{plugin::PluginEncoder, protocol::PluginResponse};

#[derive(Clone, Debug)]
pub struct JsonSerializer;

impl PluginEncoder for JsonSerializer {
    fn name(&self) -> &str {
        "json"
    }

    fn encode_call(
        &self,
        plugin_call: &crate::protocol::PluginCall,
        writer: &mut impl std::io::Write,
    ) -> Result<(), nu_protocol::ShellError> {
        serde_json::to_writer(writer, plugin_call)
            .map_err(|err| ShellError::PluginFailedToEncode(err.to_string()))
    }

    fn decode_call(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<crate::protocol::PluginCall, nu_protocol::ShellError> {
        serde_json::from_reader(reader)
            .map_err(|err| ShellError::PluginFailedToEncode(err.to_string()))
    }

    fn encode_response(
        &self,
        plugin_response: &PluginResponse,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        serde_json::to_writer(writer, plugin_response)
            .map_err(|err| ShellError::PluginFailedToEncode(err.to_string()))
    }

    fn decode_response(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<PluginResponse, ShellError> {
        serde_json::from_reader(reader)
            .map_err(|err| ShellError::PluginFailedToEncode(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        CallInfo, CallInput, EvaluatedCall, LabeledError, PluginCall, PluginData, PluginResponse,
    };
    use nu_protocol::{Signature, Span, Spanned, SyntaxShape, Value};

    #[test]
    fn callinfo_round_trip_signature() {
        let plugin_call = PluginCall::Signature;
        let encoder = JsonSerializer {};

        let mut buffer: Vec<u8> = Vec::new();
        encoder
            .encode_call(&plugin_call, &mut buffer)
            .expect("unable to serialize message");
        let returned = encoder
            .decode_call(&mut buffer.as_slice())
            .expect("unable to deserialize message");

        match returned {
            PluginCall::Signature => {}
            PluginCall::CallInfo(_) => panic!("decoded into wrong value"),
            PluginCall::CollapseCustomValue(_) => panic!("decoded into wrong value"),
        }
    }

    #[test]
    fn callinfo_round_trip_callinfo() {
        let name = "test".to_string();

        let input = Value::Bool {
            val: false,
            span: Span { start: 1, end: 20 },
        };

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
            input: CallInput::Value(input.clone()),
        });

        let encoder = JsonSerializer {};
        let mut buffer: Vec<u8> = Vec::new();
        encoder
            .encode_call(&plugin_call, &mut buffer)
            .expect("unable to serialize message");
        let returned = encoder
            .decode_call(&mut buffer.as_slice())
            .expect("unable to deserialize message");

        match returned {
            PluginCall::Signature => panic!("returned wrong call type"),
            PluginCall::CallInfo(call_info) => {
                assert_eq!(name, call_info.name);
                assert_eq!(CallInput::Value(input), call_info.input);
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

        let encoder = JsonSerializer {};
        let mut buffer: Vec<u8> = Vec::new();
        encoder
            .encode_call(&collapse_custom_value, &mut buffer)
            .expect("unable to serialize message");
        let returned = encoder
            .decode_call(&mut buffer.as_slice())
            .expect("unable to deserialize message");

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

        let encoder = JsonSerializer {};
        let mut buffer: Vec<u8> = Vec::new();
        encoder
            .encode_response(&response, &mut buffer)
            .expect("unable to serialize message");
        let returned = encoder
            .decode_response(&mut buffer.as_slice())
            .expect("unable to deserialize message");

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

        let encoder = JsonSerializer {};
        let mut buffer: Vec<u8> = Vec::new();
        encoder
            .encode_response(&response, &mut buffer)
            .expect("unable to serialize message");
        let returned = encoder
            .decode_response(&mut buffer.as_slice())
            .expect("unable to deserialize message");

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
    fn response_round_trip_plugin_data() {
        let name = "test".to_string();

        let data = vec![1, 2, 3, 4, 5];
        let span = Span { start: 2, end: 30 };

        let response = PluginResponse::PluginData(
            name.clone(),
            PluginData {
                data: data.clone(),
                span,
            },
        );

        let encoder = JsonSerializer {};
        let mut buffer: Vec<u8> = Vec::new();
        encoder
            .encode_response(&response, &mut buffer)
            .expect("unable to serialize message");
        let returned = encoder
            .decode_response(&mut buffer.as_slice())
            .expect("unable to deserialize message");

        match returned {
            PluginResponse::Error(_) => panic!("returned wrong call type"),
            PluginResponse::Signature(_) => panic!("returned wrong call type"),
            PluginResponse::Value(_) => panic!("returned wrong call type"),
            PluginResponse::PluginData(returned_name, returned_plugin_data) => {
                assert_eq!(name, returned_name);
                assert_eq!(data, returned_plugin_data.data);
                assert_eq!(span, returned_plugin_data.span);
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

        let encoder = JsonSerializer {};
        let mut buffer: Vec<u8> = Vec::new();
        encoder
            .encode_response(&response, &mut buffer)
            .expect("unable to serialize message");
        let returned = encoder
            .decode_response(&mut buffer.as_slice())
            .expect("unable to deserialize message");

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

        let encoder = JsonSerializer {};
        let mut buffer: Vec<u8> = Vec::new();
        encoder
            .encode_response(&response, &mut buffer)
            .expect("unable to serialize message");
        let returned = encoder
            .decode_response(&mut buffer.as_slice())
            .expect("unable to deserialize message");

        match returned {
            PluginResponse::Error(msg) => assert_eq!(error, msg),
            PluginResponse::Signature(_) => panic!("returned wrong call type"),
            PluginResponse::Value(_) => panic!("returned wrong call type"),
            PluginResponse::PluginData(..) => panic!("returned wrong call type"),
        }
    }
}
