macro_rules! generate_tests {
    ($encoder:expr) => {
        use nu_plugin_protocol::{
            CallInfo, CustomValueOp, EvaluatedCall, PipelineDataHeader, PluginCall,
            PluginCallResponse, PluginCustomValue, PluginInput, PluginOption, PluginOutput,
            StreamData,
        };
        use nu_protocol::{
            LabeledError, PipelineMetadata, PluginSignature, Signature, Span, Spanned, SyntaxShape,
            Value,
        };

        #[test]
        fn decode_eof() {
            let mut buffer: &[u8] = &[];
            let encoder = $encoder;
            let result: Option<PluginInput> = encoder
                .decode(&mut buffer)
                .expect("eof should not result in an error");
            assert!(result.is_none(), "decode result: {result:?}");
            let result: Option<PluginOutput> = encoder
                .decode(&mut buffer)
                .expect("eof should not result in an error");
            assert!(result.is_none(), "decode result: {result:?}");
        }

        #[test]
        fn decode_io_error() {
            struct ErrorProducer;
            impl std::io::Read for ErrorProducer {
                fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                    Err(std::io::Error::from(std::io::ErrorKind::ConnectionReset))
                }
            }
            let encoder = $encoder;
            let mut buffered = std::io::BufReader::new(ErrorProducer);
            match Encoder::<PluginInput>::decode(&encoder, &mut buffered) {
                Ok(_) => panic!("decode: i/o error was not passed through"),
                Err(ShellError::Io(_)) => (), // okay
                Err(other) => panic!(
                    "decode: got other error, should have been a \
                    ShellError::Io: {other:?}"
                ),
            }
            match Encoder::<PluginOutput>::decode(&encoder, &mut buffered) {
                Ok(_) => panic!("decode: i/o error was not passed through"),
                Err(ShellError::Io(_)) => (), // okay
                Err(other) => panic!(
                    "decode: got other error, should have been a \
                    ShellError::Io: {other:?}"
                ),
            }
        }

        #[test]
        fn decode_gibberish() {
            // just a sequence of bytes that shouldn't be valid in anything we use
            let gibberish: &[u8] = &[
                0, 80, 74, 85, 117, 122, 86, 100, 74, 115, 20, 104, 55, 98, 67, 203, 83, 85, 77,
                112, 74, 79, 254, 71, 80,
            ];
            let encoder = $encoder;

            let mut buffered = std::io::BufReader::new(&gibberish[..]);
            match Encoder::<PluginInput>::decode(&encoder, &mut buffered) {
                Ok(value) => panic!("decode: parsed successfully => {value:?}"),
                Err(ShellError::PluginFailedToDecode { .. }) => (), // okay
                Err(other) => panic!(
                    "decode: got other error, should have been a \
                    ShellError::PluginFailedToDecode: {other:?}"
                ),
            }

            let mut buffered = std::io::BufReader::new(&gibberish[..]);
            match Encoder::<PluginOutput>::decode(&encoder, &mut buffered) {
                Ok(value) => panic!("decode: parsed successfully => {value:?}"),
                Err(ShellError::PluginFailedToDecode { .. }) => (), // okay
                Err(other) => panic!(
                    "decode: got other error, should have been a \
                    ShellError::PluginFailedToDecode: {other:?}"
                ),
            }
        }

        #[test]
        fn call_round_trip_signature() {
            let plugin_call = PluginCall::Signature;
            let plugin_input = PluginInput::Call(0, plugin_call);
            let encoder = $encoder;

            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&plugin_input, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginInput::Call(0, PluginCall::Signature) => {}
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn call_round_trip_run() {
            let name = "test".to_string();

            let input = Value::bool(false, Span::new(1, 20));

            let call = EvaluatedCall {
                head: Span::new(0, 10),
                positional: vec![
                    Value::float(1.0, Span::new(0, 10)),
                    Value::string("something", Span::new(0, 10)),
                ],
                named: vec![(
                    Spanned {
                        item: "name".to_string(),
                        span: Span::new(0, 10),
                    },
                    Some(Value::float(1.0, Span::new(0, 10))),
                )],
            };

            let metadata = Some(PipelineMetadata {
                content_type: Some("foobar".into()),
                ..Default::default()
            });

            let plugin_call = PluginCall::Run(CallInfo {
                name: name.clone(),
                call: call.clone(),
                input: PipelineDataHeader::Value(input.clone(), metadata.clone()),
            });

            let plugin_input = PluginInput::Call(1, plugin_call);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&plugin_input, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginInput::Call(1, PluginCall::Run(call_info)) => {
                    assert_eq!(name, call_info.name);
                    assert_eq!(PipelineDataHeader::Value(input, metadata), call_info.input);
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
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn call_round_trip_customvalueop() {
            let data = vec![1, 2, 3, 4, 5, 6, 7];
            let span = Span::new(0, 20);

            let custom_value_op = PluginCall::CustomValueOp(
                Spanned {
                    item: PluginCustomValue::new("Foo".into(), data.clone(), false),
                    span,
                },
                CustomValueOp::ToBaseValue,
            );

            let plugin_input = PluginInput::Call(2, custom_value_op);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&plugin_input, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginInput::Call(2, PluginCall::CustomValueOp(val, op)) => {
                    assert_eq!("Foo", val.item.name());
                    assert_eq!(data, val.item.data());
                    assert_eq!(span, val.span);
                    #[allow(unreachable_patterns)]
                    match op {
                        CustomValueOp::ToBaseValue => (),
                        _ => panic!("wrong op: {op:?}"),
                    }
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn response_round_trip_signature() {
            let signature = PluginSignature::new(
                Signature::build("nu-plugin")
                    .required("first", SyntaxShape::String, "first required")
                    .required("second", SyntaxShape::Int, "second required")
                    .required_named("first-named", SyntaxShape::String, "first named", Some('f'))
                    .required_named(
                        "second-named",
                        SyntaxShape::String,
                        "second named",
                        Some('s'),
                    )
                    .rest("remaining", SyntaxShape::Int, "remaining"),
                vec![],
            );

            let response = PluginCallResponse::Signature(vec![signature.clone()]);
            let output = PluginOutput::CallResponse(3, response);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&output, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginOutput::CallResponse(
                    3,
                    PluginCallResponse::Signature(returned_signature),
                ) => {
                    assert_eq!(returned_signature.len(), 1);
                    assert_eq!(signature.sig.name, returned_signature[0].sig.name);
                    assert_eq!(
                        signature.sig.description,
                        returned_signature[0].sig.description
                    );
                    assert_eq!(
                        signature.sig.extra_description,
                        returned_signature[0].sig.extra_description
                    );
                    assert_eq!(signature.sig.is_filter, returned_signature[0].sig.is_filter);

                    signature
                        .sig
                        .required_positional
                        .iter()
                        .zip(returned_signature[0].sig.required_positional.iter())
                        .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                    signature
                        .sig
                        .optional_positional
                        .iter()
                        .zip(returned_signature[0].sig.optional_positional.iter())
                        .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                    signature
                        .sig
                        .named
                        .iter()
                        .zip(returned_signature[0].sig.named.iter())
                        .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

                    assert_eq!(
                        signature.sig.rest_positional,
                        returned_signature[0].sig.rest_positional,
                    );
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn response_round_trip_value() {
            let value = Value::int(10, Span::new(2, 30));

            let response = PluginCallResponse::value(value.clone());
            let output = PluginOutput::CallResponse(4, response);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&output, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginOutput::CallResponse(
                    4,
                    PluginCallResponse::PipelineData(PipelineDataHeader::Value(returned_value, _)),
                ) => {
                    assert_eq!(value, returned_value)
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn response_round_trip_plugin_custom_value() {
            let name = "test";

            let data = vec![1, 2, 3, 4, 5];
            let span = Span::new(2, 30);

            let value = Value::custom(
                Box::new(PluginCustomValue::new(name.into(), data.clone(), true)),
                span,
            );

            let response = PluginCallResponse::PipelineData(PipelineDataHeader::value(value));
            let output = PluginOutput::CallResponse(5, response);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&output, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginOutput::CallResponse(
                    5,
                    PluginCallResponse::PipelineData(PipelineDataHeader::Value(returned_value, _)),
                ) => {
                    assert_eq!(span, returned_value.span());

                    if let Some(plugin_val) = returned_value
                        .as_custom_value()
                        .unwrap()
                        .as_any()
                        .downcast_ref::<PluginCustomValue>()
                    {
                        assert_eq!(name, plugin_val.name());
                        assert_eq!(data, plugin_val.data());
                        assert!(plugin_val.notify_on_drop());
                    } else {
                        panic!("returned CustomValue is not a PluginCustomValue");
                    }
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn response_round_trip_error() {
            let error = LabeledError::new("label")
                .with_code("test::error")
                .with_url("https://example.org/test/error")
                .with_help("some help")
                .with_label("msg", Span::new(2, 30))
                .with_inner(ShellError::Io(IoError::new(
                    shell_error::io::ErrorKind::from_std(std::io::ErrorKind::NotFound),
                    Span::test_data(),
                    None,
                )));

            let response = PluginCallResponse::Error(error.clone());
            let output = PluginOutput::CallResponse(6, response);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&output, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginOutput::CallResponse(6, PluginCallResponse::Error(msg)) => {
                    assert_eq!(error, msg)
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn response_round_trip_error_none() {
            let error = LabeledError::new("error");
            let response = PluginCallResponse::Error(error.clone());
            let output = PluginOutput::CallResponse(7, response);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&output, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginOutput::CallResponse(7, PluginCallResponse::Error(msg)) => {
                    assert_eq!(error, msg)
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn input_round_trip_stream_data_list() {
            let span = Span::new(12, 30);
            let item = Value::int(1, span);

            let stream_data = StreamData::List(item.clone());
            let plugin_input = PluginInput::Data(0, stream_data);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&plugin_input, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginInput::Data(id, StreamData::List(list_data)) => {
                    assert_eq!(0, id);
                    assert_eq!(item, list_data);
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn input_round_trip_stream_data_raw() {
            let data = b"Hello world";

            let stream_data = StreamData::Raw(Ok(data.to_vec()));
            let plugin_input = PluginInput::Data(1, stream_data);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&plugin_input, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginInput::Data(id, StreamData::Raw(bytes)) => {
                    assert_eq!(1, id);
                    match bytes {
                        Ok(bytes) => assert_eq!(data, &bytes[..]),
                        Err(err) => panic!("decoded into error variant: {err:?}"),
                    }
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn output_round_trip_stream_data_list() {
            let span = Span::new(12, 30);
            let item = Value::int(1, span);

            let stream_data = StreamData::List(item.clone());
            let plugin_output = PluginOutput::Data(4, stream_data);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&plugin_output, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginOutput::Data(id, StreamData::List(list_data)) => {
                    assert_eq!(4, id);
                    assert_eq!(item, list_data);
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn output_round_trip_stream_data_raw() {
            let data = b"Hello world";

            let stream_data = StreamData::Raw(Ok(data.to_vec()));
            let plugin_output = PluginOutput::Data(5, stream_data);

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&plugin_output, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginOutput::Data(id, StreamData::Raw(bytes)) => {
                    assert_eq!(5, id);
                    match bytes {
                        Ok(bytes) => assert_eq!(data, &bytes[..]),
                        Err(err) => panic!("decoded into error variant: {err:?}"),
                    }
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }

        #[test]
        fn output_round_trip_option() {
            let plugin_output = PluginOutput::Option(PluginOption::GcDisabled(true));

            let encoder = $encoder;
            let mut buffer: Vec<u8> = Vec::new();
            encoder
                .encode(&plugin_output, &mut buffer)
                .expect("unable to serialize message");
            let returned = encoder
                .decode(&mut buffer.as_slice())
                .expect("unable to deserialize message")
                .expect("eof");

            match returned {
                PluginOutput::Option(PluginOption::GcDisabled(disabled)) => {
                    assert!(disabled);
                }
                _ => panic!("decoded into wrong value: {returned:?}"),
            }
        }
    };
}

pub(crate) use generate_tests;
