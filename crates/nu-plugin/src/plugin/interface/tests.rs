use crate::test_util::TestCaseExt;

use super::{EngineInterfaceManager, ReceivedPluginCall};
use nu_engine::command_prelude::IoError;
use nu_plugin_core::{Interface, InterfaceManager, interface_test_util::TestCase};
use nu_plugin_protocol::{
    ByteStreamInfo, CallInfo, CustomValueOp, EngineCall, EngineCallId, EngineCallResponse,
    EvaluatedCall, ListStreamInfo, PipelineDataHeader, PluginCall, PluginCallResponse,
    PluginCustomValue, PluginInput, PluginOutput, Protocol, ProtocolInfo, StreamData,
    test_util::{TestCustomValue, expected_test_custom_value, test_plugin_custom_value},
};
use nu_protocol::{
    BlockId, ByteStreamType, Config, CustomValue, IntoInterruptiblePipelineData, LabeledError,
    PipelineData, PluginSignature, ShellError, Signals, Span, Spanned, Value, VarId,
    engine::Closure, shell_error,
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        mpsc::{self, TryRecvError},
    },
};

#[test]
fn is_using_stdio_is_false_for_test() {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.get_interface();

    assert!(!interface.is_using_stdio());
}

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
        PipelineDataHeader::list_stream(ListStreamInfo::new(0, Span::test_data())),
        &Signals::empty(),
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
    ShellError::Io(IoError::new_with_additional_context(
        shell_error::io::ErrorKind::from_std(std::io::ErrorKind::Other),
        Span::test_data(),
        None,
        "test io error",
    ))
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
    let mut manager = test.engine();

    test.set_read_error(test_io_error());

    let stream = manager.read_pipeline_data(
        PipelineDataHeader::list_stream(ListStreamInfo::new(0, Span::test_data())),
        &Signals::empty(),
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
        PipelineDataHeader::byte_stream(ByteStreamInfo::new(
            0,
            Span::test_data(),
            ByteStreamType::Unknown,
        )),
        &Signals::empty(),
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

fn fake_engine_call(
    manager: &mut EngineInterfaceManager,
    id: EngineCallId,
) -> mpsc::Receiver<EngineCallResponse<PipelineData>> {
    // Set up a fake engine call subscription
    let (tx, rx) = mpsc::channel();

    manager.engine_call_subscriptions.insert(id, tx);

    rx
}

#[test]
fn manager_consume_all_propagates_io_error_to_engine_calls() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.engine();
    let interface = manager.get_interface();

    test.set_read_error(test_io_error());

    // Set up a fake engine call subscription
    let rx = fake_engine_call(&mut manager, 0);

    manager
        .consume_all(&mut test)
        .expect_err("consume_all did not error");

    // We have to hold interface until now otherwise consume_all won't try to process the message
    drop(interface);

    let message = rx.try_recv().expect("failed to get engine call message");
    match message {
        EngineCallResponse::Error(error) => {
            check_test_io_error(&error);
            Ok(())
        }
        _ => panic!("received something other than an error: {message:?}"),
    }
}

#[test]
fn manager_consume_all_propagates_message_error_to_engine_calls() -> Result<(), ShellError> {
    let mut test = TestCase::new();
    let mut manager = test.engine();
    let interface = manager.get_interface();

    test.add(invalid_input());

    // Set up a fake engine call subscription
    let rx = fake_engine_call(&mut manager, 0);

    manager
        .consume_all(&mut test)
        .expect_err("consume_all did not error");

    // We have to hold interface until now otherwise consume_all won't try to process the message
    drop(interface);

    let message = rx.try_recv().expect("failed to get engine call message");
    match message {
        EngineCallResponse::Error(error) => {
            check_invalid_input_error(&error);
            Ok(())
        }
        _ => panic!("received something other than an error: {message:?}"),
    }
}

#[test]
fn manager_consume_sets_protocol_info_on_hello() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();

    let info = ProtocolInfo::default();

    manager.consume(PluginInput::Hello(info.clone()))?;

    let set_info = manager
        .state
        .protocol_info
        .try_get()?
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
    assert!(!manager.state.protocol_info.is_set());

    let error = manager
        .consume(PluginInput::Drop(0))
        .expect_err("consume before Hello should cause an error");

    assert!(format!("{error:?}").contains("Hello"));
    Ok(())
}

fn set_default_protocol_info(manager: &mut EngineInterfaceManager) -> Result<(), ShellError> {
    manager
        .protocol_info_mut
        .set(Arc::new(ProtocolInfo::default()))
}

#[test]
fn manager_consume_goodbye_closes_plugin_call_channel() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    set_default_protocol_info(&mut manager)?;

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
fn manager_consume_call_metadata_forwards_to_receiver_with_context() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    set_default_protocol_info(&mut manager)?;

    let rx = manager
        .take_plugin_call_receiver()
        .expect("couldn't take receiver");

    manager.consume(PluginInput::Call(0, PluginCall::Metadata))?;

    match rx.try_recv().expect("call was not forwarded to receiver") {
        ReceivedPluginCall::Metadata { engine } => {
            assert_eq!(Some(0), engine.context);
            Ok(())
        }
        call => panic!("wrong call type: {call:?}"),
    }
}

#[test]
fn manager_consume_call_signature_forwards_to_receiver_with_context() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    set_default_protocol_info(&mut manager)?;

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
    set_default_protocol_info(&mut manager)?;

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
    set_default_protocol_info(&mut manager)?;

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
            input: PipelineDataHeader::list_stream(ListStreamInfo::new(6, Span::test_data())),
        }),
    ))?;

    for i in 0..10 {
        manager.consume(PluginInput::Data(6, Value::test_int(i).into()))?;
    }

    manager.consume(PluginInput::End(6))?;

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
    set_default_protocol_info(&mut manager)?;

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
    set_default_protocol_info(&mut manager)?;

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
            assert_eq!("TestCustomValue", custom_value.item.name());
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
fn manager_consume_engine_call_response_forwards_to_subscriber_with_pipeline_data()
-> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    set_default_protocol_info(&mut manager)?;

    let rx = fake_engine_call(&mut manager, 0);

    manager.consume(PluginInput::EngineCallResponse(
        0,
        EngineCallResponse::PipelineData(PipelineDataHeader::list_stream(ListStreamInfo::new(
            0,
            Span::test_data(),
        ))),
    ))?;

    for i in 0..2 {
        manager.consume(PluginInput::Data(0, Value::test_int(i).into()))?;
    }

    manager.consume(PluginInput::End(0))?;

    // Make sure the streams end and we don't deadlock
    drop(manager);

    let response = rx.try_recv().expect("failed to get engine call response");

    match response {
        EngineCallResponse::PipelineData(data) => {
            // Ensure we manage to receive the stream messages
            assert_eq!(2, data.into_iter().count());
            Ok(())
        }
        _ => panic!("unexpected response: {response:?}"),
    }
}

#[test]
fn manager_prepare_pipeline_data_deserializes_custom_values() -> Result<(), ShellError> {
    let manager = TestCase::new().engine();

    let data = manager.prepare_pipeline_data(PipelineData::value(
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
        .into_pipeline_data(Span::test_data(), Signals::empty()),
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

    let invalid_custom_value = PluginCustomValue::new(
        "Invalid".into(),
        vec![0; 8], // should fail to decode to anything
        false,
    );

    let span = Span::new(20, 30);
    let data = manager.prepare_pipeline_data(
        [Value::custom(Box::new(invalid_custom_value), span)]
            .into_pipeline_data(Span::test_data(), Signals::empty()),
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
        .write_response(Ok::<_, ShellError>(PipelineData::value(
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
                    PipelineDataHeader::Value(value, _) => assert_eq!(6, value.as_int()?),
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
            [Value::test_int(3), Value::test_int(4), Value::test_int(5)]
                .into_pipeline_data(Span::test_data(), Signals::empty()),
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
            PluginOutput::Data(id, data) => {
                assert_eq!(info.id, id, "Data id");
                match data {
                    StreamData::List(val) => assert_eq!(number, val.as_int()?),
                    _ => panic!("expected List data: {data:?}"),
                }
            }
            message => panic!("expected Data(..): {message:?}"),
        }
    }

    match test.next_written().expect("missing stream End message") {
        PluginOutput::End(id) => assert_eq!(info.id, id, "End id"),
        message => panic!("expected Data(..): {message:?}"),
    }

    assert!(!test.has_unconsumed_write());

    Ok(())
}

#[test]
fn interface_write_response_with_error() -> Result<(), ShellError> {
    let test = TestCase::new();
    let interface = test.engine().interface_for_context(35);
    let labeled_error = LabeledError::new("this is an error").with_help("a test error");
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
fn interface_write_engine_call_registers_subscription() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    assert!(
        manager.engine_call_subscriptions.is_empty(),
        "engine call subscriptions not empty before start of test"
    );

    let interface = manager.interface_for_context(0);
    let _ = interface.write_engine_call(EngineCall::GetConfig)?;

    manager.receive_engine_call_subscriptions();
    assert!(
        !manager.engine_call_subscriptions.is_empty(),
        "not registered"
    );
    Ok(())
}

#[test]
fn interface_write_engine_call_writes_with_correct_context() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(32);
    let _ = interface.write_engine_call(EngineCall::GetConfig)?;

    match test.next_written().expect("nothing written") {
        PluginOutput::EngineCall { context, call, .. } => {
            assert_eq!(32, context, "context incorrect");
            assert!(
                matches!(call, EngineCall::GetConfig),
                "incorrect engine call (expected GetConfig): {call:?}"
            );
        }
        other => panic!("incorrect output: {other:?}"),
    }

    assert!(!test.has_unconsumed_write());
    Ok(())
}

/// Fake responses to requests for engine call messages
fn start_fake_plugin_call_responder(
    manager: EngineInterfaceManager,
    take: usize,
    mut f: impl FnMut(EngineCallId) -> EngineCallResponse<PipelineData> + Send + 'static,
) {
    std::thread::Builder::new()
        .name("fake engine call responder".into())
        .spawn(move || {
            for (id, sub) in manager
                .engine_call_subscription_receiver
                .into_iter()
                .take(take)
            {
                sub.send(f(id)).expect("failed to send");
            }
        })
        .expect("failed to spawn thread");
}

#[test]
fn interface_get_config() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(0);

    start_fake_plugin_call_responder(manager, 1, |_| {
        EngineCallResponse::Config(Config::default().into())
    });

    let _ = interface.get_config()?;
    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_get_plugin_config() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(0);

    start_fake_plugin_call_responder(manager, 2, |id| {
        if id == 0 {
            EngineCallResponse::PipelineData(PipelineData::empty())
        } else {
            EngineCallResponse::PipelineData(PipelineData::value(Value::test_int(2), None))
        }
    });

    let first_config = interface.get_plugin_config()?;
    assert!(first_config.is_none(), "should be None: {first_config:?}");

    let second_config = interface.get_plugin_config()?;
    assert_eq!(Some(Value::test_int(2)), second_config);

    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_get_env_var() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(0);

    start_fake_plugin_call_responder(manager, 2, |id| {
        if id == 0 {
            EngineCallResponse::empty()
        } else {
            EngineCallResponse::value(Value::test_string("/foo"))
        }
    });

    let first_val = interface.get_env_var("FOO")?;
    assert!(first_val.is_none(), "should be None: {first_val:?}");

    let second_val = interface.get_env_var("FOO")?;
    assert_eq!(Some(Value::test_string("/foo")), second_val);

    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_get_current_dir() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(0);

    start_fake_plugin_call_responder(manager, 1, |_| {
        EngineCallResponse::value(Value::test_string("/current/directory"))
    });

    let val = interface.get_env_var("FOO")?;
    assert_eq!(Some(Value::test_string("/current/directory")), val);

    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_get_env_vars() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(0);

    let envs: HashMap<String, Value> = [("FOO".to_owned(), Value::test_string("foo"))]
        .into_iter()
        .collect();
    let envs_clone = envs.clone();

    start_fake_plugin_call_responder(manager, 1, move |_| {
        EngineCallResponse::ValueMap(envs_clone.clone())
    });

    let received_envs = interface.get_env_vars()?;

    assert_eq!(envs, received_envs);

    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_add_env_var() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(0);

    start_fake_plugin_call_responder(manager, 1, move |_| EngineCallResponse::empty());

    interface.add_env_var("FOO", Value::test_string("bar"))?;

    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_get_help() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(0);

    start_fake_plugin_call_responder(manager, 1, move |_| {
        EngineCallResponse::value(Value::test_string("help string"))
    });

    let help = interface.get_help()?;

    assert_eq!("help string", help);

    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_get_span_contents() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(0);

    start_fake_plugin_call_responder(manager, 1, move |_| {
        EngineCallResponse::value(Value::test_binary(b"test string"))
    });

    let contents = interface.get_span_contents(Span::test_data())?;

    assert_eq!(b"test string", &contents[..]);

    assert!(test.has_unconsumed_write());
    Ok(())
}

#[test]
fn interface_eval_closure_with_stream() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.interface_for_context(0);

    start_fake_plugin_call_responder(manager, 1, |_| {
        EngineCallResponse::PipelineData(PipelineData::value(Value::test_int(2), None))
    });

    let result = interface
        .eval_closure_with_stream(
            &Spanned {
                item: Closure {
                    block_id: BlockId::new(42),
                    captures: vec![(VarId::new(0), Value::test_int(5))],
                },
                span: Span::test_data(),
            },
            vec![Value::test_string("test")],
            PipelineData::empty(),
            true,
            false,
        )?
        .into_value(Span::test_data())?;

    assert_eq!(Value::test_int(2), result);

    // Double check the message that was written, as it's complicated
    match test.next_written().expect("nothing written") {
        PluginOutput::EngineCall { call, .. } => match call {
            EngineCall::EvalClosure {
                closure,
                positional,
                input,
                redirect_stdout,
                redirect_stderr,
            } => {
                assert_eq!(
                    BlockId::new(42),
                    closure.item.block_id,
                    "closure.item.block_id"
                );
                assert_eq!(1, closure.item.captures.len(), "closure.item.captures.len");
                assert_eq!(
                    (VarId::new(0), Value::test_int(5)),
                    closure.item.captures[0],
                    "closure.item.captures[0]"
                );
                assert_eq!(Span::test_data(), closure.span, "closure.span");
                assert_eq!(1, positional.len(), "positional.len");
                assert_eq!(Value::test_string("test"), positional[0], "positional[0]");
                assert!(matches!(input, PipelineDataHeader::Empty));
                assert!(redirect_stdout);
                assert!(!redirect_stderr);
            }
            _ => panic!("wrong engine call: {call:?}"),
        },
        other => panic!("wrong output: {other:?}"),
    }

    Ok(())
}

#[test]
fn interface_prepare_pipeline_data_serializes_custom_values() -> Result<(), ShellError> {
    let interface = TestCase::new().engine().get_interface();

    let data = interface.prepare_pipeline_data(
        PipelineData::value(
            Value::test_custom_value(Box::new(expected_test_custom_value())),
            None,
        ),
        &(),
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
    assert_eq!(expected.name(), custom_value.name());
    assert_eq!(expected.data(), custom_value.data());

    Ok(())
}

#[test]
fn interface_prepare_pipeline_data_serializes_custom_values_in_streams() -> Result<(), ShellError> {
    let interface = TestCase::new().engine().get_interface();

    let data = interface.prepare_pipeline_data(
        [Value::test_custom_value(Box::new(
            expected_test_custom_value(),
        ))]
        .into_pipeline_data(Span::test_data(), Signals::empty()),
        &(),
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
    assert_eq!(expected.name(), custom_value.name());
    assert_eq!(expected.data(), custom_value.data());

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
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "CantSerialize".into()
    }

    fn to_base_value(&self, _span: Span) -> Result<Value, ShellError> {
        unimplemented!()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[test]
fn interface_prepare_pipeline_data_embeds_serialization_errors_in_streams() -> Result<(), ShellError>
{
    let interface = TestCase::new().engine().get_interface();

    let span = Span::new(40, 60);
    let data = interface.prepare_pipeline_data(
        [Value::custom(Box::new(CantSerialize::BadVariant), span)]
            .into_pipeline_data(Span::test_data(), Signals::empty()),
        &(),
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
