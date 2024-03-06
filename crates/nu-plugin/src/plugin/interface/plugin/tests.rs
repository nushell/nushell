use std::sync::mpsc;

use nu_protocol::{
    IntoInterruptiblePipelineData, PipelineData, PluginSignature, ShellError, Span, Spanned, Value,
};

use crate::{
    plugin::{
        context::PluginExecutionBogusContext,
        interface::{test_util::TestCase, Interface, InterfaceManager},
        PluginIdentity,
    },
    protocol::{
        test_util::{expected_test_custom_value, test_plugin_custom_value},
        CallInfo, CustomValueOp, ExternalStreamInfo, ListStreamInfo, PipelineDataHeader,
        PluginCall, PluginCallId, PluginCustomValue, PluginInput, Protocol, ProtocolInfo,
        RawStreamInfo, StreamData, StreamMessage,
    },
    EvaluatedCall, PluginCallResponse, PluginOutput,
};

use super::{
    PluginCallSubscription, PluginInterface, PluginInterfaceManager, ReceivedPluginCallMessage,
};

#[test]
fn manager_consume_all_consumes_messages() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.plugin("test");

    // This message should be non-problematic
    test.add(PluginOutput::Hello(ProtocolInfo::default()));

    manager.consume_all(&mut test)?;

    assert!(!test.has_unconsumed_read());
    Ok(())
}

#[test]
fn manager_consume_all_exits_after_streams_and_interfaces_are_dropped() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.plugin("test");

    // Add messages that won't cause errors
    for _ in 0..5 {
        test.add(PluginOutput::Hello(ProtocolInfo::default()));
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
fn manager_consume_all_propagates_io_error_to_readers() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.plugin("test");

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

fn invalid_output() -> PluginOutput {
    // This should definitely cause an error, as 0.0.0 is not compatible with any version other than
    // itself
    PluginOutput::Hello(ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "0.0.0".into(),
        features: vec![],
    })
}

fn check_invalid_output_error(error: &ShellError) {
    // the error message should include something about the version...
    assert!(format!("{error:?}").contains("0.0.0"), "error: {error}");
}

#[test]
fn manager_consume_all_propagates_message_error_to_readers() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.plugin("test");

    test.add(invalid_output());

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
        check_invalid_output_error(&error);
        Ok(())
    } else {
        panic!("did not get an error");
    }
}

fn fake_plugin_call(
    manager: &mut PluginInterfaceManager,
    id: PluginCallId,
) -> mpsc::Receiver<ReceivedPluginCallMessage> {
    // Set up a fake plugin call subscription
    let (tx, rx) = mpsc::channel();

    manager.plugin_call_subscriptions.insert(
        id,
        PluginCallSubscription {
            sender: tx,
            context: None,
        },
    );

    rx
}

#[test]
fn manager_consume_all_propagates_io_error_to_plugin_calls() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.plugin("test");
    let interface = manager.get_interface();

    test.set_read_error(test_io_error());

    // Set up a fake plugin call subscription
    let rx = fake_plugin_call(&mut manager, 0);

    manager
        .consume_all(&mut test)
        .expect_err("consume_all did not error");

    // We have to hold interface until now otherwise consume_all won't try to process the message
    drop(interface);

    let message = rx.try_recv().expect("failed to get plugin call message");
    match message {
        ReceivedPluginCallMessage::Error(error) => {
            check_test_io_error(&error);
            Ok(())
        }
        _ => panic!("received something other than an error: {message:?}"),
    }
}

#[test]
fn manager_consume_all_propagates_message_error_to_plugin_calls() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.plugin("test");
    let interface = manager.get_interface();

    test.add(invalid_output());

    // Set up a fake plugin call subscription
    let rx = fake_plugin_call(&mut manager, 0);

    manager
        .consume_all(&mut test)
        .expect_err("consume_all did not error");

    // We have to hold interface until now otherwise consume_all won't try to process the message
    drop(interface);

    let message = rx.try_recv().expect("failed to get plugin call message");
    match message {
        ReceivedPluginCallMessage::Error(error) => {
            check_invalid_output_error(&error);
            Ok(())
        }
        _ => panic!("received something other than an error: {message:?}"),
    }
}

#[test]
fn manager_consume_sets_protocol_info_on_hello() -> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");

    let info = ProtocolInfo::default();

    manager.consume(PluginOutput::Hello(info.clone()))?;

    let set_info = manager
        .protocol_info
        .as_ref()
        .expect("protocol info not set");
    assert_eq!(info.version, set_info.version);
    Ok(())
}

#[test]
fn manager_consume_errors_on_wrong_nushell_version() -> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");

    let info = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "0.0.0".into(),
        features: vec![],
    };

    manager
        .consume(PluginOutput::Hello(info))
        .expect_err("version 0.0.0 should cause an error");
    Ok(())
}

#[test]
fn manager_consume_errors_on_sending_other_messages_before_hello() -> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");

    // hello not set
    assert!(manager.protocol_info.is_none());

    let error = manager
        .consume(PluginOutput::Stream(StreamMessage::Drop(0)))
        .expect_err("consume before Hello should cause an error");

    assert!(format!("{error:?}").contains("Hello"));
    Ok(())
}

#[test]
fn manager_consume_call_response_forwards_to_subscriber_with_pipeline_data(
) -> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");
    manager.protocol_info = Some(ProtocolInfo::default());

    let rx = fake_plugin_call(&mut manager, 0);

    manager.consume(PluginOutput::CallResponse(
        0,
        PluginCallResponse::PipelineData(PipelineDataHeader::ListStream(ListStreamInfo { id: 0 })),
    ))?;

    for i in 0..2 {
        manager.consume(PluginOutput::Stream(StreamMessage::Data(
            0,
            Value::test_int(i).into(),
        )))?;
    }

    manager.consume(PluginOutput::Stream(StreamMessage::End(0)))?;

    // Make sure the streams end and we don't deadlock
    drop(manager);

    let message = rx
        .try_recv()
        .expect("failed to get plugin call response message");

    match message {
        ReceivedPluginCallMessage::Response(response) => match response {
            PluginCallResponse::PipelineData(data) => {
                // Ensure we manage to receive the stream messages
                assert_eq!(2, data.into_iter().count());
                Ok(())
            }
            _ => panic!("unexpected response: {response:?}"),
        },
        _ => panic!("unexpected response message: {message:?}"),
    }
}

#[test]
fn manager_prepare_pipeline_data_adds_source_to_values() -> Result<(), ShellError> {
    let manager = TestCase::new().plugin("test");

    let data = manager.prepare_pipeline_data(PipelineData::Value(
        Value::test_custom_value(Box::new(test_plugin_custom_value())),
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
        .expect("custom value is not a PluginCustomValue");

    if let Some(source) = &custom_value.source {
        assert_eq!("test", source.plugin_name);
    } else {
        panic!("source was not set");
    }

    Ok(())
}

#[test]
fn manager_prepare_pipeline_data_adds_source_to_list_streams() -> Result<(), ShellError> {
    let manager = TestCase::new().plugin("test");

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
    let custom_value: &PluginCustomValue = value
        .as_custom_value()?
        .as_any()
        .downcast_ref()
        .expect("custom value is not a PluginCustomValue");

    if let Some(source) = &custom_value.source {
        assert_eq!("test", source.plugin_name);
    } else {
        panic!("source was not set");
    }

    Ok(())
}

#[test]
fn interface_hello_sends_protocol_info() -> Result<(), ShellError> {
    let test = TestCase::new();
    let interface = test.plugin("test").get_interface();
    interface.hello()?;

    let written = test.next_written().expect("nothing written");

    match written {
        PluginInput::Hello(info) => {
            assert_eq!(ProtocolInfo::default().version, info.version);
        }
        _ => panic!("unexpected message written: {written:?}"),
    }

    assert!(!test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_goodbye() -> Result<(), ShellError> {
    let test = TestCase::new();
    let interface = test.plugin("test").get_interface();
    interface.goodbye()?;

    let written = test.next_written().expect("nothing written");

    assert!(
        matches!(written, PluginInput::Goodbye),
        "not goodbye: {written:?}"
    );

    assert!(!test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_write_plugin_call_registers_subscription() -> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");
    assert!(
        manager.plugin_call_subscriptions.is_empty(),
        "plugin call subscriptions not empty before start of test"
    );

    let interface = manager.get_interface();
    let _ = interface.write_plugin_call(PluginCall::Signature, None)?;

    manager.receive_plugin_call_subscriptions();
    assert!(
        !manager.plugin_call_subscriptions.is_empty(),
        "not registered"
    );
    Ok(())
}

#[test]
fn interface_write_plugin_call_writes_signature() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.plugin("test");
    let interface = manager.get_interface();

    let (writer, _) = interface.write_plugin_call(PluginCall::Signature, None)?;
    writer.write()?;

    let written = test.next_written().expect("nothing written");
    match written {
        PluginInput::Call(_, call) => assert!(
            matches!(call, PluginCall::Signature),
            "not Signature: {call:?}"
        ),
        _ => panic!("unexpected message written: {written:?}"),
    }
    Ok(())
}

#[test]
fn interface_write_plugin_call_writes_custom_value_op() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.plugin("test");
    let interface = manager.get_interface();

    let (writer, _) = interface.write_plugin_call(
        PluginCall::CustomValueOp(
            Spanned {
                item: test_plugin_custom_value(),
                span: Span::test_data(),
            },
            CustomValueOp::ToBaseValue,
        ),
        None,
    )?;
    writer.write()?;

    let written = test.next_written().expect("nothing written");
    match written {
        PluginInput::Call(_, call) => assert!(
            matches!(
                call,
                PluginCall::CustomValueOp(_, CustomValueOp::ToBaseValue)
            ),
            "expected CustomValueOp(_, ToBaseValue), got {call:?}"
        ),
        _ => panic!("unexpected message written: {written:?}"),
    }
    Ok(())
}

#[test]
fn interface_write_plugin_call_writes_run_with_value_input() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.plugin("test");
    let interface = manager.get_interface();

    let (writer, _) = interface.write_plugin_call(
        PluginCall::Run(CallInfo {
            name: "foo".into(),
            call: EvaluatedCall {
                head: Span::test_data(),
                positional: vec![],
                named: vec![],
            },
            input: PipelineData::Value(Value::test_int(-1), None),
            config: None,
        }),
        None,
    )?;
    writer.write()?;

    let written = test.next_written().expect("nothing written");
    match written {
        PluginInput::Call(_, call) => match call {
            PluginCall::Run(CallInfo { name, input, .. }) => {
                assert_eq!("foo", name);
                match input {
                    PipelineDataHeader::Value(value) => assert_eq!(-1, value.as_int()?),
                    _ => panic!("unexpected input header: {input:?}"),
                }
            }
            _ => panic!("unexpected Call: {call:?}"),
        },
        _ => panic!("unexpected message written: {written:?}"),
    }
    Ok(())
}

#[test]
fn interface_write_plugin_call_writes_run_with_stream_input() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.plugin("test");
    let interface = manager.get_interface();

    let values = vec![Value::test_int(1), Value::test_int(2)];
    let (writer, _) = interface.write_plugin_call(
        PluginCall::Run(CallInfo {
            name: "foo".into(),
            call: EvaluatedCall {
                head: Span::test_data(),
                positional: vec![],
                named: vec![],
            },
            input: values.clone().into_pipeline_data(None),
            config: None,
        }),
        None,
    )?;
    writer.write()?;

    let written = test.next_written().expect("nothing written");
    let info = match written {
        PluginInput::Call(_, call) => match call {
            PluginCall::Run(CallInfo { name, input, .. }) => {
                assert_eq!("foo", name);
                match input {
                    PipelineDataHeader::ListStream(info) => info,
                    _ => panic!("unexpected input header: {input:?}"),
                }
            }
            _ => panic!("unexpected Call: {call:?}"),
        },
        _ => panic!("unexpected message written: {written:?}"),
    };

    // Expect stream messages
    for value in values {
        match test
            .next_written()
            .expect("failed to get Data stream message")
        {
            PluginInput::Stream(StreamMessage::Data(id, data)) => {
                assert_eq!(info.id, id, "id");
                match data {
                    StreamData::List(data_value) => {
                        assert_eq!(value, data_value, "wrong value in Data message");
                    }
                    _ => panic!("not List stream data: {data:?}"),
                }
            }
            message => panic!("expected Stream(Data(..)) message: {message:?}"),
        }
    }

    match test
        .next_written()
        .expect("failed to get End stream message")
    {
        PluginInput::Stream(StreamMessage::End(id)) => {
            assert_eq!(info.id, id, "id");
        }
        message => panic!("expected Stream(End(_)) message: {message:?}"),
    }

    Ok(())
}

#[test]
fn interface_receive_plugin_call_receives_response() -> Result<(), ShellError> {
    let interface = TestCase::new().plugin("test").get_interface();

    // Set up a fake channel that has the response in it
    let (tx, rx) = mpsc::channel();
    tx.send(ReceivedPluginCallMessage::Response(
        PluginCallResponse::Signature(vec![]),
    ))
    .expect("failed to send on new channel");
    drop(tx); // so we don't deadlock on recv()

    let response = interface.receive_plugin_call_response(rx)?;
    assert!(
        matches!(response, PluginCallResponse::Signature(_)),
        "wrong response: {response:?}"
    );
    Ok(())
}

#[test]
fn interface_receive_plugin_call_receives_error() -> Result<(), ShellError> {
    let interface = TestCase::new().plugin("test").get_interface();

    // Set up a fake channel that has the error in it
    let (tx, rx) = mpsc::channel();
    tx.send(ReceivedPluginCallMessage::Error(
        ShellError::ExternalNotSupported {
            span: Span::test_data(),
        },
    ))
    .expect("failed to send on new channel");
    drop(tx); // so we don't deadlock on recv()

    let error = interface
        .receive_plugin_call_response(rx)
        .expect_err("did not receive error");
    assert!(
        matches!(error, ShellError::ExternalNotSupported { .. }),
        "wrong error: {error:?}"
    );
    Ok(())
}

/// Fake responses to requests for plugin call messages
fn start_fake_plugin_call_responder(
    manager: PluginInterfaceManager,
    take: usize,
    mut f: impl FnMut(PluginCallId) -> Vec<ReceivedPluginCallMessage> + Send + 'static,
) {
    std::thread::Builder::new()
        .name("fake plugin call responder".into())
        .spawn(move || {
            for (id, sub) in manager
                .plugin_call_subscription_receiver
                .into_iter()
                .take(take)
            {
                for message in f(id) {
                    sub.sender.send(message).expect("failed to send");
                }
            }
        })
        .expect("failed to spawn thread");
}

#[test]
fn interface_get_signature() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.plugin("test");
    let interface = manager.get_interface();

    start_fake_plugin_call_responder(manager, 1, |_| {
        vec![ReceivedPluginCallMessage::Response(
            PluginCallResponse::Signature(vec![PluginSignature::build("test")]),
        )]
    });

    let signatures = interface.get_signature()?;

    assert_eq!(1, signatures.len());
    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_run() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.plugin("test");
    let interface = manager.get_interface();
    let number = 64;

    start_fake_plugin_call_responder(manager, 1, move |_| {
        vec![ReceivedPluginCallMessage::Response(
            PluginCallResponse::PipelineData(PipelineData::Value(Value::test_int(number), None)),
        )]
    });

    let result = interface.run(
        CallInfo {
            name: "bogus".into(),
            call: EvaluatedCall {
                head: Span::test_data(),
                positional: vec![],
                named: vec![],
            },
            input: PipelineData::Empty,
            config: None,
        },
        PluginExecutionBogusContext.into(),
    )?;

    assert_eq!(
        Value::test_int(number),
        result.into_value(Span::test_data())
    );
    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_custom_value_to_base_value() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.plugin("test");
    let interface = manager.get_interface();
    let string = "this is a test";

    start_fake_plugin_call_responder(manager, 1, move |_| {
        vec![ReceivedPluginCallMessage::Response(
            PluginCallResponse::PipelineData(PipelineData::Value(Value::test_string(string), None)),
        )]
    });

    let result = interface.custom_value_to_base_value(Spanned {
        item: test_plugin_custom_value(),
        span: Span::test_data(),
    })?;

    assert_eq!(Value::test_string(string), result);
    assert!(test.has_unconsumed_write());
    Ok(())
}

fn normal_values(interface: &PluginInterface) -> Vec<Value> {
    vec![
        Value::test_int(5),
        Value::test_custom_value(Box::new(PluginCustomValue {
            name: "SomeTest".into(),
            data: vec![1, 2, 3],
            // Has the same source, so it should be accepted
            source: Some(interface.state.identity.clone()),
        })),
    ]
}

#[test]
fn interface_prepare_pipeline_data_accepts_normal_values() -> Result<(), ShellError> {
    let interface = TestCase::new().plugin("test").get_interface();
    for value in normal_values(&interface) {
        match interface.prepare_pipeline_data(PipelineData::Value(value.clone(), None)) {
            Ok(data) => assert_eq!(
                value.get_type(),
                data.into_value(Span::test_data()).get_type()
            ),
            Err(err) => panic!("failed to accept {value:?}: {err}"),
        }
    }
    Ok(())
}

#[test]
fn interface_prepare_pipeline_data_accepts_normal_streams() -> Result<(), ShellError> {
    let interface = TestCase::new().plugin("test").get_interface();
    let values = normal_values(&interface);
    let data = interface.prepare_pipeline_data(values.clone().into_pipeline_data(None))?;

    let mut count = 0;
    for (expected_value, actual_value) in values.iter().zip(data) {
        assert!(
            !actual_value.is_error(),
            "error value instead of {expected_value:?} in stream: {actual_value:?}"
        );
        assert_eq!(expected_value.get_type(), actual_value.get_type());
        count += 1;
    }
    assert_eq!(
        values.len(),
        count,
        "didn't receive as many values as expected"
    );
    Ok(())
}

fn bad_custom_values() -> Vec<Value> {
    // These shouldn't be accepted
    vec![
        // Native custom value (not PluginCustomValue) should be rejected
        Value::test_custom_value(Box::new(expected_test_custom_value())),
        // Has no source, so it should be rejected
        Value::test_custom_value(Box::new(PluginCustomValue {
            name: "SomeTest".into(),
            data: vec![1, 2, 3],
            source: None,
        })),
        // Has a different source, so it should be rejected
        Value::test_custom_value(Box::new(PluginCustomValue {
            name: "SomeTest".into(),
            data: vec![1, 2, 3],
            source: Some(PluginIdentity::new_fake("pluto")),
        })),
    ]
}

#[test]
fn interface_prepare_pipeline_data_rejects_bad_custom_value() -> Result<(), ShellError> {
    let interface = TestCase::new().plugin("test").get_interface();
    for value in bad_custom_values() {
        match interface.prepare_pipeline_data(PipelineData::Value(value.clone(), None)) {
            Err(err) => match err {
                ShellError::CustomValueIncorrectForPlugin { .. } => (),
                _ => panic!("expected error type CustomValueIncorrectForPlugin, but got {err:?}"),
            },
            Ok(_) => panic!("mistakenly accepted {value:?}"),
        }
    }
    Ok(())
}

#[test]
fn interface_prepare_pipeline_data_rejects_bad_custom_value_in_a_stream() -> Result<(), ShellError>
{
    let interface = TestCase::new().plugin("test").get_interface();
    let values = bad_custom_values();
    let data = interface.prepare_pipeline_data(values.clone().into_pipeline_data(None))?;

    let mut count = 0;
    for value in data {
        assert!(value.is_error(), "expected error value for {value:?}");
        count += 1;
    }
    assert_eq!(
        values.len(),
        count,
        "didn't receive as many values as expected"
    );
    Ok(())
}
