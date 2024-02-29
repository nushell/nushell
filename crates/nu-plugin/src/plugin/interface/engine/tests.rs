use std::sync::mpsc::TryRecvError;

use nu_protocol::{
    CustomValue, IntoInterruptiblePipelineData, PipelineData, PluginSignature, ShellError, Span,
    Spanned, Value,
};

use crate::{
    plugin::interface::{test_util::TestCase, Interface, InterfaceManager},
    protocol::{
        test_util::{expected_test_custom_value, test_plugin_custom_value, TestCustomValue},
        CallInfo, CustomValueOp, ExternalStreamInfo, ListStreamInfo, PipelineDataHeader,
        PluginCall, PluginCustomValue, PluginInput, Protocol, ProtocolInfo, RawStreamInfo,
        StreamData, StreamMessage,
    },
    EvaluatedCall, LabeledError, PluginCallResponse, PluginOutput,
};

use super::ReceivedPluginCall;

#[test]
fn manager_consume_all_consumes_messages() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.engine();

    // This message should be non-problematic
    test.add(PluginInput::Hello(ProtocolInfo::default()));

    manager.consume_all(&mut test)?;

    assert!(!test.has_unconsumed_read());
    Ok(())
}

#[test]
fn manager_consume_all_exits_after_streams_and_interfaces_are_dropped() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.engine();

    // Add messages that won't cause errors
    for _ in 0..5 {
        test.add(PluginInput::Hello(ProtocolInfo::default()));
    }

    // Create a stream...
    let stream = manager.read_pipeline_data(
        PipelineDataHeader::ListStream(ListStreamInfo { id: 0 }),
        None,
    )?;

    // and an interface...
    let interface = manager.get_interface();

    // Expect that is_finished is false
    assert!(
        !manager.is_finished(),
        "is_finished is true even though active stream/interface exists"
    );

    // After dropping, it should be true
    drop(stream);
    drop(interface);

    assert!(
        manager.is_finished(),
        "is_finished is false even though manager has no stream or interface"
    );

    // When it's true, consume_all shouldn't consume everything
    manager.consume_all(&mut test)?;

    assert!(
        test.has_unconsumed_read(),
        "consume_all consumed the messages"
    );
    Ok(())
}

fn test_io_error() -> ShellError {
    ShellError::IOError {
        msg: "test io error".into(),
    }
}

fn check_test_io_error(error: &ShellError) {
    assert!(
        format!("{error:?}").contains("test io error"),
        "error: {error}"
    );
}

#[test]
fn manager_consume_all_propagates_error_to_readers() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.engine();

    test.set_read_error(test_io_error());

    let stream = manager.read_pipeline_data(
        PipelineDataHeader::ListStream(ListStreamInfo { id: 0 }),
        None,
    )?;

    manager
        .consume_all(&mut test)
        .expect_err("consume_all did not error");

    // Ensure end of stream
    drop(manager);

    let value = stream.into_iter().next().expect("stream is empty");
    if let Value::Error { error, .. } = value {
        check_test_io_error(&error);
        Ok(())
    } else {
        panic!("did not get an error");
    }
}

fn invalid_input() -> PluginInput {
    // This should definitely cause an error, as 0.0.0 is not compatible with any version other than
    // itself
    PluginInput::Hello(ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "0.0.0".into(),
        features: vec![],
    })
}

fn check_invalid_input_error(error: &ShellError) {
    // the error message should include something about the version...
    assert!(format!("{error:?}").contains("0.0.0"), "error: {error}");
}

#[test]
fn manager_consume_all_propagates_message_error_to_readers() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.engine();

    test.add(invalid_input());

    let stream = manager.read_pipeline_data(
        PipelineDataHeader::ExternalStream(ExternalStreamInfo {
            span: Span::test_data(),
            stdout: Some(RawStreamInfo {
                id: 0,
                is_binary: false,
                known_size: None,
            }),
            stderr: None,
            exit_code: None,
            trim_end_newline: false,
        }),
        None,
    )?;

    manager
        .consume_all(&mut test)
        .expect_err("consume_all did not error");

    // Ensure end of stream
    drop(manager);

    let value = stream.into_iter().next().expect("stream is empty");
    if let Value::Error { error, .. } = value {
        check_invalid_input_error(&error);
        Ok(())
    } else {
        panic!("did not get an error");
    }
}

#[test]
fn manager_consume_sets_protocol_info_on_hello() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();

    let info = ProtocolInfo::default();

    manager.consume(PluginInput::Hello(info.clone()))?;

    let set_info = manager
        .protocol_info
        .as_ref()
        .expect("protocol info not set");
    assert_eq!(info.version, set_info.version);
    Ok(())
}

#[test]
fn manager_consume_errors_on_wrong_nushell_version() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();

    let info = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "0.0.0".into(),
        features: vec![],
    };

    manager
        .consume(PluginInput::Hello(info))
        .expect_err("version 0.0.0 should cause an error");
    Ok(())
}

#[test]
fn manager_consume_errors_on_sending_other_messages_before_hello() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();

    // hello not set
    assert!(manager.protocol_info.is_none());

    let error = manager
        .consume(PluginInput::Stream(StreamMessage::Drop(0)))
        .expect_err("consume before Hello should cause an error");

    assert!(format!("{error:?}").contains("Hello"));
    Ok(())
}

#[test]
fn manager_consume_goodbye_closes_plugin_call_channel() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    manager.protocol_info = Some(ProtocolInfo::default());

    let rx = manager
        .take_plugin_call_receiver()
        .expect("plugin call receiver missing");

    manager.consume(PluginInput::Goodbye)?;

    match rx.try_recv() {
        Err(TryRecvError::Disconnected) => (),
        _ => panic!("receiver was not disconnected"),
    }

    Ok(())
}

#[test]
fn manager_consume_call_signature_forwards_to_receiver_with_context() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    manager.protocol_info = Some(ProtocolInfo::default());

    let rx = manager
        .take_plugin_call_receiver()
        .expect("couldn't take receiver");

    manager.consume(PluginInput::Call(0, PluginCall::Signature))?;

    match rx.try_recv().expect("call was not forwarded to receiver") {
        ReceivedPluginCall::Signature { engine } => {
            assert_eq!(Some(0), engine.context);
            Ok(())
        }
        call => panic!("wrong call type: {call:?}"),
    }
}

#[test]
fn manager_consume_call_run_forwards_to_receiver_with_context() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    manager.protocol_info = Some(ProtocolInfo::default());

    let rx = manager
        .take_plugin_call_receiver()
        .expect("couldn't take receiver");

    manager.consume(PluginInput::Call(
        17,
        PluginCall::Run(CallInfo {
            name: "bar".into(),
            call: EvaluatedCall {
                head: Span::test_data(),
                positional: vec![],
                named: vec![],
            },
            input: PipelineDataHeader::Empty,
            config: None,
        }),
    ))?;

    // Make sure the streams end and we don't deadlock
    drop(manager);

    match rx.try_recv().expect("call was not forwarded to receiver") {
        ReceivedPluginCall::Run { engine, call: _ } => {
            assert_eq!(Some(17), engine.context, "context");
            Ok(())
        }
        call => panic!("wrong call type: {call:?}"),
    }
}

#[test]
fn manager_consume_call_run_forwards_to_receiver_with_pipeline_data() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    manager.protocol_info = Some(ProtocolInfo::default());

    let rx = manager
        .take_plugin_call_receiver()
        .expect("couldn't take receiver");

    manager.consume(PluginInput::Call(
        0,
        PluginCall::Run(CallInfo {
            name: "bar".into(),
            call: EvaluatedCall {
                head: Span::test_data(),
                positional: vec![],
                named: vec![],
            },
            input: PipelineDataHeader::ListStream(ListStreamInfo { id: 6 }),
            config: None,
        }),
    ))?;

    for i in 0..10 {
        manager.consume(PluginInput::Stream(StreamMessage::Data(
            6,
            Value::test_int(i).into(),
        )))?;
    }

    manager.consume(PluginInput::Stream(StreamMessage::End(6)))?;

    // Make sure the streams end and we don't deadlock
    drop(manager);

    match rx.try_recv().expect("call was not forwarded to receiver") {
        ReceivedPluginCall::Run { engine: _, call } => {
            assert_eq!("bar", call.name);
            // Ensure we manage to receive the stream messages
            assert_eq!(10, call.input.into_iter().count());
            Ok(())
        }
        call => panic!("wrong call type: {call:?}"),
    }
}

#[test]
fn manager_consume_call_run_deserializes_custom_values_in_args() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    manager.protocol_info = Some(ProtocolInfo::default());

    let rx = manager
        .take_plugin_call_receiver()
        .expect("couldn't take receiver");

    let value = Value::test_custom_value(Box::new(test_plugin_custom_value()));

    manager.consume(PluginInput::Call(
        0,
        PluginCall::Run(CallInfo {
            name: "bar".into(),
            call: EvaluatedCall {
                head: Span::test_data(),
                positional: vec![value.clone()],
                named: vec![(
                    Spanned {
                        item: "flag".into(),
                        span: Span::test_data(),
                    },
                    Some(value),
                )],
            },
            input: PipelineDataHeader::Empty,
            config: None,
        }),
    ))?;

    // Make sure the streams end and we don't deadlock
    drop(manager);

    match rx.try_recv().expect("call was not forwarded to receiver") {
        ReceivedPluginCall::Run { engine: _, call } => {
            assert_eq!(1, call.call.positional.len());
            assert_eq!(1, call.call.named.len());

            for arg in call.call.positional {
                let custom_value: &TestCustomValue = arg
                    .as_custom_value()?
                    .as_any()
                    .downcast_ref()
                    .expect("positional arg is not TestCustomValue");
                assert_eq!(expected_test_custom_value(), *custom_value, "positional");
            }

            for (key, val) in call.call.named {
                let key = &key.item;
                let custom_value: &TestCustomValue = val
                    .as_ref()
                    .unwrap_or_else(|| panic!("found empty named argument: {key}"))
                    .as_custom_value()?
                    .as_any()
                    .downcast_ref()
                    .unwrap_or_else(|| panic!("named arg {key} is not TestCustomValue"));
                assert_eq!(expected_test_custom_value(), *custom_value, "named: {key}");
            }

            Ok(())
        }
        call => panic!("wrong call type: {call:?}"),
    }
}

#[test]
fn manager_consume_call_custom_value_op_forwards_to_receiver_with_context() -> Result<(), ShellError>
{
    let mut manager = TestCase::new().engine();
    manager.protocol_info = Some(ProtocolInfo::default());

    let rx = manager
        .take_plugin_call_receiver()
        .expect("couldn't take receiver");

    manager.consume(PluginInput::Call(
        32,
        PluginCall::CustomValueOp(
            Spanned {
                item: test_plugin_custom_value(),
                span: Span::test_data(),
            },
            CustomValueOp::ToBaseValue,
        ),
    ))?;

    match rx.try_recv().expect("call was not forwarded to receiver") {
        ReceivedPluginCall::CustomValueOp {
            engine,
            custom_value,
            op,
        } => {
            assert_eq!(Some(32), engine.context);
            assert_eq!("TestCustomValue", custom_value.item.name);
            assert!(
                matches!(op, CustomValueOp::ToBaseValue),
                "incorrect op: {op:?}"
            );
        }
        call => panic!("wrong call type: {call:?}"),
    }

    Ok(())
}

#[test]
fn manager_prepare_pipeline_data_deserializes_custom_values() -> Result<(), ShellError> {
    let manager = TestCase::new().engine();

    let data = manager.prepare_pipeline_data(PipelineData::Value(
        Value::test_custom_value(Box::new(test_plugin_custom_value())),
        None,
    ))?;

    let value = data
        .into_iter()
        .next()
        .expect("prepared pipeline data is empty");
    let custom_value: &TestCustomValue = value
        .as_custom_value()?
        .as_any()
        .downcast_ref()
        .expect("custom value is not a TestCustomValue, probably not deserialized");

    assert_eq!(expected_test_custom_value(), *custom_value);

    Ok(())
}

#[test]
fn manager_prepare_pipeline_data_deserializes_custom_values_in_streams() -> Result<(), ShellError> {
    let manager = TestCase::new().engine();

    let data = manager.prepare_pipeline_data(
        [Value::test_custom_value(Box::new(
            test_plugin_custom_value(),
        ))]
        .into_pipeline_data(None),
    )?;

    let value = data
        .into_iter()
        .next()
        .expect("prepared pipeline data is empty");
    let custom_value: &TestCustomValue = value
        .as_custom_value()?
        .as_any()
        .downcast_ref()
        .expect("custom value is not a TestCustomValue, probably not deserialized");

    assert_eq!(expected_test_custom_value(), *custom_value);

    Ok(())
}

#[test]
fn manager_prepare_pipeline_data_embeds_deserialization_errors_in_streams() -> Result<(), ShellError>
{
    let manager = TestCase::new().engine();

    let invalid_custom_value = PluginCustomValue {
        name: "Invalid".into(),
        data: vec![0; 8], // should fail to decode to anything
        source: None,
    };

    let span = Span::new(20, 30);
    let data = manager.prepare_pipeline_data(
        [Value::custom_value(Box::new(invalid_custom_value), span)].into_pipeline_data(None),
    )?;

    let value = data
        .into_iter()
        .next()
        .expect("prepared pipeline data is empty");

    match value {
        Value::Error { error, .. } => match *error {
            ShellError::CustomValueFailedToDecode {
                span: error_span, ..
            } => {
                assert_eq!(span, error_span, "error span not the same as the value's");
            }
            _ => panic!("expected ShellError::CustomValueFailedToDecode, but got {error:?}"),
        },
        _ => panic!("unexpected value, not error: {value:?}"),
    }

    Ok(())
}

#[test]
fn interface_hello_sends_protocol_info() -> Result<(), ShellError> {
    let test = TestCase::new();
    let interface = test.engine().get_interface();
    interface.hello()?;

    let written = test.next_written().expect("nothing written");

    match written {
        PluginOutput::Hello(info) => {
            assert_eq!(ProtocolInfo::default().version, info.version);
        }
        _ => panic!("unexpected message written: {written:?}"),
    }

    assert!(!test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_write_response_with_value() -> Result<(), ShellError> {
    let test = TestCase::new();
    let interface = test.engine().interface_for_context(33);
    interface
        .write_response(Ok::<_, ShellError>(PipelineData::Value(
            Value::test_int(6),
            None,
        )))?
        .write()?;

    let written = test.next_written().expect("nothing written");

    match written {
        PluginOutput::CallResponse(id, response) => {
            assert_eq!(33, id, "id");
            match response {
                PluginCallResponse::PipelineData(header) => match header {
                    PipelineDataHeader::Value(value) => assert_eq!(6, value.as_int()?),
                    _ => panic!("unexpected pipeline data header: {header:?}"),
                },
                _ => panic!("unexpected response: {response:?}"),
            }
        }
        _ => panic!("unexpected message written: {written:?}"),
    }

    assert!(!test.has_unconsumed_write());

    Ok(())
}

#[test]
fn interface_write_response_with_stream() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(34);

    interface
        .write_response(Ok::<_, ShellError>(
            [Value::test_int(3), Value::test_int(4), Value::test_int(5)].into_pipeline_data(None),
        ))?
        .write()?;

    let written = test.next_written().expect("nothing written");

    let info = match written {
        PluginOutput::CallResponse(_, response) => match response {
            PluginCallResponse::PipelineData(header) => match header {
                PipelineDataHeader::ListStream(info) => info,
                _ => panic!("expected ListStream header: {header:?}"),
            },
            _ => panic!("wrong response: {response:?}"),
        },
        _ => panic!("wrong output written: {written:?}"),
    };

    for number in [3, 4, 5] {
        match test.next_written().expect("missing stream Data message") {
            PluginOutput::Stream(StreamMessage::Data(id, data)) => {
                assert_eq!(info.id, id, "Data id");
                match data {
                    StreamData::List(val) => assert_eq!(number, val.as_int()?),
                    _ => panic!("expected List data: {data:?}"),
                }
            }
            message => panic!("expected Stream(Data(..)): {message:?}"),
        }
    }

    match test.next_written().expect("missing stream End message") {
        PluginOutput::Stream(StreamMessage::End(id)) => assert_eq!(info.id, id, "End id"),
        message => panic!("expected Stream(Data(..)): {message:?}"),
    }

    assert!(!test.has_unconsumed_write());

    Ok(())
}

#[test]
fn interface_write_response_with_error() -> Result<(), ShellError> {
    let test = TestCase::new();
    let interface = test.engine().interface_for_context(35);
    let labeled_error = LabeledError {
        label: "this is an error".into(),
        msg: "a test error".into(),
        span: None,
    };
    interface
        .write_response(Err(labeled_error.clone()))?
        .write()?;

    let written = test.next_written().expect("nothing written");

    match written {
        PluginOutput::CallResponse(id, response) => {
            assert_eq!(35, id, "id");
            match response {
                PluginCallResponse::Error(err) => assert_eq!(labeled_error, err),
                _ => panic!("unexpected response: {response:?}"),
            }
        }
        _ => panic!("unexpected message written: {written:?}"),
    }

    assert!(!test.has_unconsumed_write());

    Ok(())
}

#[test]
fn interface_write_signature() -> Result<(), ShellError> {
    let test = TestCase::new();
    let interface = test.engine().interface_for_context(36);
    let signatures = vec![PluginSignature::build("test command")];
    interface.write_signature(signatures.clone())?;

    let written = test.next_written().expect("nothing written");

    match written {
        PluginOutput::CallResponse(id, response) => {
            assert_eq!(36, id, "id");
            match response {
                PluginCallResponse::Signature(sigs) => assert_eq!(1, sigs.len(), "sigs.len"),
                _ => panic!("unexpected response: {response:?}"),
            }
        }
        _ => panic!("unexpected message written: {written:?}"),
    }

    assert!(!test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_prepare_pipeline_data_serializes_custom_values() -> Result<(), ShellError> {
    let interface = TestCase::new().engine().get_interface();

    let data = interface.prepare_pipeline_data(PipelineData::Value(
        Value::test_custom_value(Box::new(expected_test_custom_value())),
        None,
    ))?;

    let value = data
        .into_iter()
        .next()
        .expect("prepared pipeline data is empty");
    let custom_value: &PluginCustomValue = value
        .as_custom_value()?
        .as_any()
        .downcast_ref()
        .expect("custom value is not a PluginCustomValue, probably not serialized");

    let expected = test_plugin_custom_value();
    assert_eq!(expected.name, custom_value.name);
    assert_eq!(expected.data, custom_value.data);
    assert!(custom_value.source.is_none());

    Ok(())
}

#[test]
fn interface_prepare_pipeline_data_serializes_custom_values_in_streams() -> Result<(), ShellError> {
    let interface = TestCase::new().engine().get_interface();

    let data = interface.prepare_pipeline_data(
        [Value::test_custom_value(Box::new(
            expected_test_custom_value(),
        ))]
        .into_pipeline_data(None),
    )?;

    let value = data
        .into_iter()
        .next()
        .expect("prepared pipeline data is empty");
    let custom_value: &PluginCustomValue = value
        .as_custom_value()?
        .as_any()
        .downcast_ref()
        .expect("custom value is not a PluginCustomValue, probably not serialized");

    let expected = test_plugin_custom_value();
    assert_eq!(expected.name, custom_value.name);
    assert_eq!(expected.data, custom_value.data);
    assert!(custom_value.source.is_none());

    Ok(())
}

/// A non-serializable custom value. Should cause a serialization error
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum CantSerialize {
    #[serde(skip_serializing)]
    BadVariant,
}

#[typetag::serde]
impl CustomValue for CantSerialize {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn value_string(&self) -> String {
        "CantSerialize".into()
    }

    fn to_base_value(&self, _span: Span) -> Result<Value, ShellError> {
        unimplemented!()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn interface_prepare_pipeline_data_embeds_serialization_errors_in_streams() -> Result<(), ShellError>
{
    let interface = TestCase::new().engine().get_interface();

    let span = Span::new(40, 60);
    let data = interface.prepare_pipeline_data(
        [Value::custom_value(
            Box::new(CantSerialize::BadVariant),
            span,
        )]
        .into_pipeline_data(None),
    )?;

    let value = data
        .into_iter()
        .next()
        .expect("prepared pipeline data is empty");

    match value {
        Value::Error { error, .. } => match *error {
            ShellError::CustomValueFailedToEncode {
                span: error_span, ..
            } => {
                assert_eq!(span, error_span, "error span not the same as the value's");
            }
            _ => panic!("expected ShellError::CustomValueFailedToEncode, but got {error:?}"),
        },
        _ => panic!("unexpected value, not error: {value:?}"),
    }

    Ok(())
}
