use crate::plugin::{CallInfo, PluginCall, PluginError, PluginResponse};
use crate::plugin_capnp::{plugin_call, plugin_response};
use crate::serializers::{call, signature, value};
use capnp::serialize_packed;

pub fn encode_call(
    plugin_call: &PluginCall,
    writer: &mut impl std::io::Write,
) -> Result<(), PluginError> {
    let mut message = ::capnp::message::Builder::new_default();

    let mut builder = message.init_root::<plugin_call::Builder>();

    match &plugin_call {
        PluginCall::Signature => builder.set_signature(()),
        PluginCall::CallInfo(call_info) => {
            let mut call_info_builder = builder.reborrow().init_call_info();

            // Serializing argument information from the call
            let call_builder = call_info_builder
                .reborrow()
                .get_call()
                .map_err(|e| PluginError::EncodingError(e.to_string()))?;

            call::serialize_call(&call_info.call, call_builder)
                .map_err(|e| PluginError::EncodingError(e.to_string()))?;

            // Serializing the input value from the call info
            let value_builder = call_info_builder
                .reborrow()
                .get_input()
                .map_err(|e| PluginError::EncodingError(e.to_string()))?;

            value::serialize_value(&call_info.input, value_builder);
        }
    };

    serialize_packed::write_message(writer, &message)
        .map_err(|e| PluginError::EncodingError(e.to_string()))
}

pub fn decode_call(reader: &mut impl std::io::BufRead) -> Result<PluginCall, PluginError> {
    let message_reader =
        serialize_packed::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

    let reader = message_reader
        .get_root::<plugin_call::Reader>()
        .map_err(|e| PluginError::DecodingError(e.to_string()))?;

    match reader.which() {
        Err(capnp::NotInSchema(_)) => Err(PluginError::DecodingError("value not in schema".into())),
        Ok(plugin_call::Signature(())) => Ok(PluginCall::Signature),
        Ok(plugin_call::CallInfo(reader)) => {
            let reader = reader.map_err(|e| PluginError::DecodingError(e.to_string()))?;

            let call_reader = reader
                .get_call()
                .map_err(|e| PluginError::DecodingError(e.to_string()))?;

            let call = call::deserialize_call(call_reader)
                .map_err(|e| PluginError::DecodingError(e.to_string()))?;

            let input_reader = reader
                .get_input()
                .map_err(|e| PluginError::DecodingError(e.to_string()))?;

            let input = value::deserialize_value(input_reader)
                .map_err(|e| PluginError::DecodingError(e.to_string()))?;

            Ok(PluginCall::CallInfo(Box::new(CallInfo { call, input })))
        }
    }
}

pub fn encode_response(
    plugin_response: &PluginResponse,
    writer: &mut impl std::io::Write,
) -> Result<(), PluginError> {
    let mut message = ::capnp::message::Builder::new_default();

    let mut builder = message.init_root::<plugin_response::Builder>();

    match &plugin_response {
        PluginResponse::Signature(sign) => {
            let signature_builder = builder.reborrow().init_signature();
            signature::serialize_signature(sign, signature_builder)
        }
        PluginResponse::Value(val) => {
            let value_builder = builder.reborrow().init_value();
            value::serialize_value(val, value_builder);
        }
    };

    serialize_packed::write_message(writer, &message)
        .map_err(|e| PluginError::EncodingError(e.to_string()))
}

pub fn decode_response(reader: &mut impl std::io::BufRead) -> Result<PluginResponse, PluginError> {
    let message_reader =
        serialize_packed::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

    let reader = message_reader
        .get_root::<plugin_response::Reader>()
        .map_err(|e| PluginError::DecodingError(e.to_string()))?;

    match reader.which() {
        Err(capnp::NotInSchema(_)) => Err(PluginError::DecodingError("value not in schema".into())),
        Ok(plugin_response::Signature(reader)) => {
            let reader = reader.map_err(|e| PluginError::DecodingError(e.to_string()))?;
            let sign = signature::deserialize_signature(reader)
                .map_err(|e| PluginError::DecodingError(e.to_string()))?;

            Ok(PluginResponse::Signature(Box::new(sign)))
        }
        Ok(plugin_response::Value(reader)) => {
            let reader = reader.map_err(|e| PluginError::DecodingError(e.to_string()))?;
            let val = value::deserialize_value(reader)
                .map_err(|e| PluginError::DecodingError(e.to_string()))?;

            Ok(PluginResponse::Value(Box::new(val)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{PluginCall, PluginResponse};
    use nu_protocol::{
        ast::{Call, Expr, Expression},
        Signature, Span, Spanned, SyntaxShape, Value,
    };

    fn compare_expressions(lhs: &Expression, rhs: &Expression) {
        match (&lhs.expr, &rhs.expr) {
            (Expr::Bool(a), Expr::Bool(b)) => assert_eq!(a, b),
            (Expr::Int(a), Expr::Int(b)) => assert_eq!(a, b),
            (Expr::Float(a), Expr::Float(b)) => assert_eq!(a, b),
            (Expr::String(a), Expr::String(b)) => assert_eq!(a, b),
            _ => panic!("not matching values"),
        }
    }

    #[test]
    fn callinfo_round_trip_signature() {
        let plugin_call = PluginCall::Signature;

        let mut buffer: Vec<u8> = Vec::new();
        encode_call(&plugin_call, &mut buffer).expect("unable to serialize message");
        let returned = decode_call(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginCall::Signature => {}
            PluginCall::CallInfo(_) => panic!("decoded into wrong value"),
        }
    }

    #[test]
    fn callinfo_round_trip_callinfo() {
        let input = Value::Bool {
            val: false,
            span: Span { start: 1, end: 20 },
        };

        let call = Call {
            decl_id: 1,
            head: Span { start: 0, end: 10 },
            positional: vec![
                Expression {
                    expr: Expr::Float(1.0),
                    span: Span { start: 0, end: 10 },
                    ty: nu_protocol::Type::Float,
                    custom_completion: None,
                },
                Expression {
                    expr: Expr::String("something".into()),
                    span: Span { start: 0, end: 10 },
                    ty: nu_protocol::Type::Float,
                    custom_completion: None,
                },
            ],
            named: vec![(
                Spanned {
                    item: "name".to_string(),
                    span: Span { start: 0, end: 10 },
                },
                Some(Expression {
                    expr: Expr::Float(1.0),
                    span: Span { start: 0, end: 10 },
                    ty: nu_protocol::Type::Float,
                    custom_completion: None,
                }),
            )],
        };

        let plugin_call = PluginCall::CallInfo(Box::new(CallInfo {
            call: call.clone(),
            input: input.clone(),
        }));

        let mut buffer: Vec<u8> = Vec::new();
        encode_call(&plugin_call, &mut buffer).expect("unable to serialize message");
        let returned = decode_call(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginCall::Signature => panic!("returned wrong call type"),
            PluginCall::CallInfo(call_info) => {
                assert_eq!(input, call_info.input);
                assert_eq!(call.head, call_info.call.head);
                assert_eq!(call.positional.len(), call_info.call.positional.len());

                call.positional
                    .iter()
                    .zip(call_info.call.positional.iter())
                    .for_each(|(lhs, rhs)| compare_expressions(lhs, rhs));

                call.named
                    .iter()
                    .zip(call_info.call.named.iter())
                    .for_each(|(lhs, rhs)| {
                        // Comparing the keys
                        assert_eq!(lhs.0.item, rhs.0.item);

                        match (&lhs.1, &rhs.1) {
                            (None, None) => {}
                            (Some(a), Some(b)) => compare_expressions(a, b),
                            _ => panic!("not matching values"),
                        }
                    });
            }
        }
    }

    #[test]
    fn response_round_trip_signature() {
        let signature = Signature::build("nu-plugin")
            .required("first", SyntaxShape::String, "first required")
            .required("second", SyntaxShape::Int, "second required")
            .required_named("first_named", SyntaxShape::String, "first named", Some('f'))
            .required_named(
                "second_named",
                SyntaxShape::String,
                "second named",
                Some('s'),
            )
            .rest("remaining", SyntaxShape::Int, "remaining");

        let response = PluginResponse::Signature(Box::new(signature.clone()));

        let mut buffer: Vec<u8> = Vec::new();
        encode_response(&response, &mut buffer).expect("unable to serialize message");
        let returned =
            decode_response(&mut buffer.as_slice()).expect("unable to deserialize message");

        match returned {
            PluginResponse::Value(_) => panic!("returned wrong call type"),
            PluginResponse::Signature(returned_signature) => {
                assert_eq!(signature.name, returned_signature.name);
                assert_eq!(signature.usage, returned_signature.usage);
                assert_eq!(signature.extra_usage, returned_signature.extra_usage);
                assert_eq!(signature.is_filter, returned_signature.is_filter);

                signature
                    .required_positional
                    .iter()
                    .zip(returned_signature.required_positional.iter())
                    .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                signature
                    .optional_positional
                    .iter()
                    .zip(returned_signature.optional_positional.iter())
                    .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                signature
                    .named
                    .iter()
                    .zip(returned_signature.named.iter())
                    .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                assert_eq!(
                    signature.rest_positional,
                    returned_signature.rest_positional,
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
            PluginResponse::Signature(_) => panic!("returned wrong call type"),
            PluginResponse::Value(returned_value) => {
                assert_eq!(&value, returned_value.as_ref())
            }
        }
    }
}
