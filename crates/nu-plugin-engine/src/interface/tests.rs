use super::{
    Context, PluginCallState, PluginInterface, PluginInterfaceManager, ReceivedPluginCallMessage,
};
use crate::{
    PluginCustomValueWithSource, PluginSource, context::PluginExecutionBogusContext,
    interface::CurrentCallState, plugin_custom_value_with_source::WithSource, test_util::*,
};
use nu_engine::command_prelude::IoError;
use nu_plugin_core::{Interface, InterfaceManager, interface_test_util::TestCase};
use nu_plugin_protocol::{
    ByteStreamInfo, CallInfo, CustomValueOp, EngineCall, EngineCallResponse, EvaluatedCall,
    ListStreamInfo, PipelineDataHeader, PluginCall, PluginCallId, PluginCallResponse,
    PluginCustomValue, PluginInput, PluginOutput, Protocol, ProtocolInfo, StreamData,
    StreamMessage,
    test_util::{expected_test_custom_value, test_plugin_custom_value},
};
use nu_protocol::{
    BlockId, ByteStreamType, CustomValue, DataSource, IntoInterruptiblePipelineData, IntoSpanned,
    PipelineData, PipelineMetadata, PluginMetadata, PluginSignature, ShellError, Signals, Span,
    Spanned, Value,
    ast::{Math, Operator},
    engine::Closure,
    shell_error,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, mpsc},
    time::Duration,
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
    let mut manager = test.plugin("test");

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

    manager.plugin_call_states.insert(
        id,
        PluginCallState {
            sender: Some(tx),
            dont_send_response: false,
            signals: Signals::empty(),
            context_rx: None,
            span: None,
            keep_plugin_custom_values: mpsc::channel(),
            remaining_streams_to_read: 0,
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

    let message = rx.try_recv().expect("failed to get plugin call message");
    match message {
        ReceivedPluginCallMessage::Error(error) => {
            check_test_io_error(&error);
        }
        _ => panic!("received something other than an error: {message:?}"),
    }

    // Check that further calls also cause the error
    match interface.get_signature() {
        Ok(_) => panic!("plugin call after exit did not cause error somehow"),
        Err(err) => {
            check_test_io_error(&err);
            Ok(())
        }
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

    let message = rx.try_recv().expect("failed to get plugin call message");
    match message {
        ReceivedPluginCallMessage::Error(error) => {
            check_invalid_output_error(&error);
        }
        _ => panic!("received something other than an error: {message:?}"),
    }

    // Check that further calls also cause the error
    match interface.get_signature() {
        Ok(_) => panic!("plugin call after exit did not cause error somehow"),
        Err(err) => {
            check_invalid_output_error(&err);
            Ok(())
        }
    }
}

#[test]
fn manager_consume_sets_protocol_info_on_hello() -> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");

    let info = ProtocolInfo::default();

    manager.consume(PluginOutput::Hello(info.clone()))?;

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
    assert!(!manager.state.protocol_info.is_set());

    let error = manager
        .consume(PluginOutput::Drop(0))
        .expect_err("consume before Hello should cause an error");

    assert!(format!("{error:?}").contains("Hello"));
    Ok(())
}

fn set_default_protocol_info(manager: &mut PluginInterfaceManager) -> Result<(), ShellError> {
    manager
        .protocol_info_mut
        .set(Arc::new(ProtocolInfo::default()))
}

#[test]
fn manager_consume_call_response_forwards_to_subscriber_with_pipeline_data()
-> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");
    set_default_protocol_info(&mut manager)?;

    let rx = fake_plugin_call(&mut manager, 0);

    manager.consume(PluginOutput::CallResponse(
        0,
        PluginCallResponse::PipelineData(PipelineDataHeader::list_stream(ListStreamInfo::new(
            0,
            Span::test_data(),
        ))),
    ))?;

    for i in 0..2 {
        manager.consume(PluginOutput::Data(0, Value::test_int(i).into()))?;
    }

    manager.consume(PluginOutput::End(0))?;

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
fn manager_consume_call_response_registers_streams() -> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");
    set_default_protocol_info(&mut manager)?;

    for n in [0, 1] {
        fake_plugin_call(&mut manager, n);
    }

    // Check list streams, byte streams
    manager.consume(PluginOutput::CallResponse(
        0,
        PluginCallResponse::PipelineData(PipelineDataHeader::list_stream(ListStreamInfo::new(
            0,
            Span::test_data(),
        ))),
    ))?;
    manager.consume(PluginOutput::CallResponse(
        1,
        PluginCallResponse::PipelineData(PipelineDataHeader::byte_stream(ByteStreamInfo::new(
            1,
            Span::test_data(),
            ByteStreamType::Unknown,
        ))),
    ))?;

    // ListStream should have one
    if let Some(sub) = manager.plugin_call_states.get(&0) {
        assert_eq!(
            1, sub.remaining_streams_to_read,
            "ListStream remaining_streams_to_read should be 1"
        );
    } else {
        panic!("failed to find subscription for ListStream (0), maybe it was removed");
    }
    assert_eq!(
        Some(&0),
        manager.plugin_call_input_streams.get(&0),
        "plugin_call_input_streams[0] should be Some(0)"
    );

    // ByteStream should have one
    if let Some(sub) = manager.plugin_call_states.get(&1) {
        assert_eq!(
            1, sub.remaining_streams_to_read,
            "ByteStream remaining_streams_to_read should be 1"
        );
    } else {
        panic!("failed to find subscription for ByteStream (1), maybe it was removed");
    }
    assert_eq!(
        Some(&1),
        manager.plugin_call_input_streams.get(&1),
        "plugin_call_input_streams[1] should be Some(1)"
    );

    Ok(())
}

#[test]
fn manager_consume_engine_call_forwards_to_subscriber_with_pipeline_data() -> Result<(), ShellError>
{
    let mut manager = TestCase::new().plugin("test");
    set_default_protocol_info(&mut manager)?;

    let rx = fake_plugin_call(&mut manager, 37);

    manager.consume(PluginOutput::EngineCall {
        context: 37,
        id: 46,
        call: EngineCall::EvalClosure {
            closure: Spanned {
                item: Closure {
                    block_id: BlockId::new(0),
                    captures: vec![],
                },
                span: Span::test_data(),
            },
            positional: vec![],
            input: PipelineDataHeader::list_stream(ListStreamInfo::new(2, Span::test_data())),
            redirect_stdout: false,
            redirect_stderr: false,
        },
    })?;

    for i in 0..2 {
        manager.consume(PluginOutput::Data(2, Value::test_int(i).into()))?;
    }
    manager.consume(PluginOutput::End(2))?;

    // Make sure the streams end and we don't deadlock
    drop(manager);

    let message = rx.try_recv().expect("failed to get plugin call message");

    match message {
        ReceivedPluginCallMessage::EngineCall(id, call) => {
            assert_eq!(46, id, "id");
            match call {
                EngineCall::EvalClosure { input, .. } => {
                    // Count the stream messages
                    assert_eq!(2, input.into_iter().count());
                    Ok(())
                }
                _ => panic!("unexpected call: {call:?}"),
            }
        }
        _ => panic!("unexpected response message: {message:?}"),
    }
}

#[test]
fn manager_handle_engine_call_after_response_received() -> Result<(), ShellError> {
    let test = TestCase::new();
    let mut manager = test.plugin("test");
    set_default_protocol_info(&mut manager)?;

    let (context_tx, context_rx) = mpsc::channel();

    // Set up a situation identical to what we would find if the response had been read, but there
    // was still a stream being processed. We have nowhere to send the engine call in that case,
    // so the manager has to create a place to handle it.
    manager.plugin_call_states.insert(
        0,
        PluginCallState {
            sender: None,
            dont_send_response: false,
            signals: Signals::empty(),
            context_rx: Some(context_rx),
            span: None,
            keep_plugin_custom_values: mpsc::channel(),
            remaining_streams_to_read: 1,
        },
    );

    // The engine will get the context from the channel
    let bogus = Context(Box::new(PluginExecutionBogusContext));
    context_tx.send(bogus).expect("failed to send");

    manager.send_engine_call(0, 0, EngineCall::GetConfig)?;

    // Not really much choice but to wait here, as the thread will have been spawned in the
    // background; we don't have a way to know if it's executed
    let mut waited = 0;
    while !test.has_unconsumed_write() {
        if waited > 100 {
            panic!("nothing written before timeout, expected engine call response");
        } else {
            std::thread::sleep(Duration::from_millis(1));
            waited += 1;
        }
    }

    // The GetConfig call on bogus should result in an error response being written
    match test.next_written().expect("nothing written") {
        PluginInput::EngineCallResponse(id, resp) => {
            assert_eq!(0, id, "id");
            match resp {
                EngineCallResponse::Error(err) => {
                    assert!(err.to_string().contains("bogus"), "wrong error: {err}");
                }
                _ => panic!("unexpected engine call response, expected error: {resp:?}"),
            }
        }
        other => panic!("unexpected message, not engine call response: {other:?}"),
    }

    // Whatever was used to make this happen should have been held onto, since spawning a thread
    // is expensive
    let sender = &manager
        .plugin_call_states
        .get(&0)
        .expect("missing subscription 0")
        .sender;

    assert!(
        sender.is_some(),
        "failed to keep spawned engine call handler channel"
    );
    Ok(())
}

#[test]
fn manager_send_plugin_call_response_removes_context_only_if_no_streams_to_read()
-> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");

    for n in [0, 1] {
        manager.plugin_call_states.insert(
            n,
            PluginCallState {
                sender: None,
                dont_send_response: false,
                signals: Signals::empty(),
                context_rx: None,
                span: None,
                keep_plugin_custom_values: mpsc::channel(),
                remaining_streams_to_read: n as i32,
            },
        );
    }

    for n in [0, 1] {
        manager.send_plugin_call_response(n, PluginCallResponse::Signature(vec![]))?;
    }

    // 0 should not still be present, but 1 should be
    assert!(
        !manager.plugin_call_states.contains_key(&0),
        "didn't clean up when there weren't remaining streams"
    );
    assert!(
        manager.plugin_call_states.contains_key(&1),
        "clean up even though there were remaining streams"
    );
    Ok(())
}

#[test]
fn manager_consume_stream_end_removes_context_only_if_last_stream() -> Result<(), ShellError> {
    let mut manager = TestCase::new().plugin("test");
    set_default_protocol_info(&mut manager)?;

    for n in [1, 2] {
        manager.plugin_call_states.insert(
            n,
            PluginCallState {
                sender: None,
                dont_send_response: false,
                signals: Signals::empty(),
                context_rx: None,
                span: None,
                keep_plugin_custom_values: mpsc::channel(),
                remaining_streams_to_read: n as i32,
            },
        );
    }

    // 1 owns [10], 2 owns [21, 22]
    manager.plugin_call_input_streams.insert(10, 1);
    manager.plugin_call_input_streams.insert(21, 2);
    manager.plugin_call_input_streams.insert(22, 2);

    // Register the streams so we don't have errors
    let streams: Vec<_> = [10, 21, 22]
        .into_iter()
        .map(|id| {
            let interface = manager.get_interface();
            manager
                .stream_manager
                .get_handle()
                .read_stream::<Value, _>(id, interface)
        })
        .collect();

    // Ending 10 should cause 1 to be removed
    manager.consume(StreamMessage::End(10).into())?;
    assert!(
        !manager.plugin_call_states.contains_key(&1),
        "contains(1) after End(10)"
    );

    // Ending 21 should not cause 2 to be removed
    manager.consume(StreamMessage::End(21).into())?;
    assert!(
        manager.plugin_call_states.contains_key(&2),
        "!contains(2) after End(21)"
    );

    // Ending 22 should cause 2 to be removed
    manager.consume(StreamMessage::End(22).into())?;
    assert!(
        !manager.plugin_call_states.contains_key(&2),
        "contains(2) after End(22)"
    );

    drop(streams);
    Ok(())
}

#[test]
fn manager_prepare_pipeline_data_adds_source_to_values() -> Result<(), ShellError> {
    let manager = TestCase::new().plugin("test");

    let data = manager.prepare_pipeline_data(PipelineData::value(
        Value::test_custom_value(Box::new(test_plugin_custom_value())),
        None,
    ))?;

    let value = data
        .into_iter()
        .next()
        .expect("prepared pipeline data is empty");
    let custom_value: &PluginCustomValueWithSource = value
        .as_custom_value()?
        .as_any()
        .downcast_ref()
        .expect("{value:?} is not a PluginCustomValueWithSource");

    assert_eq!("test", custom_value.source().name());

    Ok(())
}

#[test]
fn manager_prepare_pipeline_data_adds_source_to_list_streams() -> Result<(), ShellError> {
    let manager = TestCase::new().plugin("test");

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
    let custom_value: &PluginCustomValueWithSource = value
        .as_custom_value()?
        .as_any()
        .downcast_ref()
        .expect("{value:?} is not a PluginCustomValueWithSource");

    assert_eq!("test", custom_value.source().name());

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
        manager.plugin_call_states.is_empty(),
        "plugin call subscriptions not empty before start of test"
    );

    let interface = manager.get_interface();
    let _ = interface.write_plugin_call(PluginCall::Signature, None)?;

    manager.receive_plugin_call_subscriptions();
    assert!(!manager.plugin_call_states.is_empty(), "not registered");
    Ok(())
}

#[test]
fn interface_write_plugin_call_writes_signature() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.plugin("test");
    let interface = manager.get_interface();

    let result = interface.write_plugin_call(PluginCall::Signature, None)?;
    result.writer.write()?;

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

    let result = interface.write_plugin_call(
        PluginCall::CustomValueOp(
            Spanned {
                item: test_plugin_custom_value(),
                span: Span::test_data(),
            },
            CustomValueOp::ToBaseValue,
        ),
        None,
    )?;
    result.writer.write()?;

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

    let metadata0 = PipelineMetadata {
        data_source: DataSource::None,
        content_type: Some("baz".into()),
    };

    let result = interface.write_plugin_call(
        PluginCall::Run(CallInfo {
            name: "foo".into(),
            call: EvaluatedCall {
                head: Span::test_data(),
                positional: vec![],
                named: vec![],
            },
            input: PipelineData::value(Value::test_int(-1), Some(metadata0.clone())),
        }),
        None,
    )?;
    result.writer.write()?;

    let written = test.next_written().expect("nothing written");
    match written {
        PluginInput::Call(_, call) => match call {
            PluginCall::Run(CallInfo { name, input, .. }) => {
                assert_eq!("foo", name);
                match input {
                    PipelineDataHeader::Value(value, metadata) => {
                        assert_eq!(-1, value.as_int()?);
                        assert_eq!(metadata0, metadata.expect("there should be metadata"));
                    }
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
    let result = interface.write_plugin_call(
        PluginCall::Run(CallInfo {
            name: "foo".into(),
            call: EvaluatedCall {
                head: Span::test_data(),
                positional: vec![],
                named: vec![],
            },
            input: values
                .clone()
                .into_pipeline_data(Span::test_data(), Signals::empty()),
        }),
        None,
    )?;
    result.writer.write()?;

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
            PluginInput::Data(id, data) => {
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
        PluginInput::End(id) => {
            assert_eq!(info.id, id, "id");
        }
        message => panic!("expected End(_) message: {message:?}"),
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

    let response = interface.receive_plugin_call_response(rx, None, CurrentCallState::default())?;
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
        .receive_plugin_call_response(rx, None, CurrentCallState::default())
        .expect_err("did not receive error");
    assert!(
        matches!(error, ShellError::ExternalNotSupported { .. }),
        "wrong error: {error:?}"
    );
    Ok(())
}

#[test]
fn interface_receive_plugin_call_handles_engine_call() -> Result<(), ShellError> {
    let test = TestCase::new();
    let interface = test.plugin("test").get_interface();

    // Set up a fake channel just for the engine call
    let (tx, rx) = mpsc::channel();
    tx.send(ReceivedPluginCallMessage::EngineCall(
        0,
        EngineCall::GetConfig,
    ))
    .expect("failed to send on new channel");

    // The context should be a bogus context, which will return an error for GetConfig
    let mut context = PluginExecutionBogusContext;

    // We don't actually send a response, so `receive_plugin_call_response` should actually return
    // an error, but it should still do the engine call
    drop(tx);
    interface
        .receive_plugin_call_response(rx, Some(&mut context), CurrentCallState::default())
        .expect_err("no error even though there was no response");

    // Check for the engine call response output
    match test
        .next_written()
        .expect("no engine call response written")
    {
        PluginInput::EngineCallResponse(id, resp) => {
            assert_eq!(0, id, "id");
            match resp {
                EngineCallResponse::Error(err) => {
                    assert!(err.to_string().contains("bogus"), "wrong error: {err}");
                }
                _ => panic!("unexpected engine call response, maybe bogus is wrong: {resp:?}"),
            }
        }
        other => panic!("unexpected message: {other:?}"),
    }
    assert!(!test.has_unconsumed_write());
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
            for (id, state) in manager
                .plugin_call_subscription_receiver
                .into_iter()
                .take(take)
            {
                for message in f(id) {
                    state
                        .sender
                        .as_ref()
                        .expect("sender was not set")
                        .send(message)
                        .expect("failed to send");
                }
            }
        })
        .expect("failed to spawn thread");
}

#[test]
fn interface_get_metadata() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.plugin("test");
    let interface = manager.get_interface();

    start_fake_plugin_call_responder(manager, 1, |_| {
        vec![ReceivedPluginCallMessage::Response(
            PluginCallResponse::Metadata(PluginMetadata::new().with_version("test")),
        )]
    });

    let metadata = interface.get_metadata()?;

    assert_eq!(Some("test"), metadata.version.as_deref());
    assert!(test.has_unconsumed_write());
    Ok(())
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
            PluginCallResponse::PipelineData(PipelineData::value(Value::test_int(number), None)),
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
            input: PipelineData::empty(),
        },
        &mut PluginExecutionBogusContext,
    )?;

    assert_eq!(
        Value::test_int(number),
        result.into_value(Span::test_data())?,
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
            PluginCallResponse::PipelineData(PipelineData::value(Value::test_string(string), None)),
        )]
    });

    let result = interface.custom_value_to_base_value(Spanned {
        item: test_plugin_custom_value_with_source(),
        span: Span::test_data(),
    })?;

    assert_eq!(Value::test_string(string), result);
    assert!(test.has_unconsumed_write());
    Ok(())
}

fn normal_values(interface: &PluginInterface) -> Vec<Value> {
    vec![
        Value::test_int(5),
        Value::test_custom_value(Box::new(
            PluginCustomValue::new("SomeTest".into(), vec![1, 2, 3], false).with_source(
                // Has the same source, so it should be accepted
                interface.state.source.clone(),
            ),
        )),
    ]
}

#[test]
fn interface_prepare_pipeline_data_accepts_normal_values() -> Result<(), ShellError> {
    let interface = TestCase::new().plugin("test").get_interface();
    let state = CurrentCallState::default();
    for value in normal_values(&interface) {
        match interface.prepare_pipeline_data(PipelineData::value(value.clone(), None), &state) {
            Ok(data) => assert_eq!(
                value.get_type(),
                data.into_value(Span::test_data())?.get_type(),
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
    let state = CurrentCallState::default();
    let data = interface.prepare_pipeline_data(
        values
            .clone()
            .into_pipeline_data(Span::test_data(), Signals::empty()),
        &state,
    )?;

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
        Value::test_custom_value(Box::new(PluginCustomValue::new(
            "SomeTest".into(),
            vec![1, 2, 3],
            false,
        ))),
        // Has a different source, so it should be rejected
        Value::test_custom_value(Box::new(
            PluginCustomValue::new("SomeTest".into(), vec![1, 2, 3], false)
                .with_source(PluginSource::new_fake("pluto").into()),
        )),
    ]
}

#[test]
fn interface_prepare_pipeline_data_rejects_bad_custom_value() -> Result<(), ShellError> {
    let interface = TestCase::new().plugin("test").get_interface();
    let state = CurrentCallState::default();
    for value in bad_custom_values() {
        match interface.prepare_pipeline_data(PipelineData::value(value.clone(), None), &state) {
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
    let state = CurrentCallState::default();
    let data = interface.prepare_pipeline_data(
        values
            .clone()
            .into_pipeline_data(Span::test_data(), Signals::empty()),
        &state,
    )?;

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

#[test]
fn prepare_custom_value_verifies_source() {
    let span = Span::test_data();
    let source = Arc::new(PluginSource::new_fake("test"));

    let mut val: Box<dyn CustomValue> = Box::new(test_plugin_custom_value());
    assert!(
        CurrentCallState::default()
            .prepare_custom_value(
                Spanned {
                    item: &mut val,
                    span,
                },
                &source
            )
            .is_err()
    );

    let mut val: Box<dyn CustomValue> =
        Box::new(test_plugin_custom_value().with_source(source.clone()));
    assert!(
        CurrentCallState::default()
            .prepare_custom_value(
                Spanned {
                    item: &mut val,
                    span,
                },
                &source
            )
            .is_ok()
    );
}

#[derive(Debug, Serialize, Deserialize)]
struct DropCustomVal;
#[typetag::serde]
impl CustomValue for DropCustomVal {
    fn clone_value(&self, _span: Span) -> Value {
        unimplemented!()
    }

    fn type_name(&self) -> String {
        "DropCustomVal".into()
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

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}

#[test]
fn prepare_custom_value_sends_to_keep_channel_if_drop_notify() -> Result<(), ShellError> {
    let span = Span::test_data();
    let source = Arc::new(PluginSource::new_fake("test"));
    let (tx, rx) = mpsc::channel();
    let state = CurrentCallState {
        keep_plugin_custom_values_tx: Some(tx),
        ..Default::default()
    };
    // Try with a custom val that has drop check set
    let mut drop_val: Box<dyn CustomValue> = Box::new(
        PluginCustomValue::serialize_from_custom_value(&DropCustomVal, span)?
            .with_source(source.clone()),
    );
    state.prepare_custom_value(
        Spanned {
            item: &mut drop_val,
            span,
        },
        &source,
    )?;
    // Check that the custom value was actually sent
    assert!(rx.try_recv().is_ok());
    // Now try with one that doesn't have it
    let mut not_drop_val: Box<dyn CustomValue> =
        Box::new(test_plugin_custom_value().with_source(source.clone()));
    state.prepare_custom_value(
        Spanned {
            item: &mut not_drop_val,
            span,
        },
        &source,
    )?;
    // Should not have been sent to the channel
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[test]
fn prepare_plugin_call_run() {
    // Check that args are handled
    let span = Span::test_data();
    let source = Arc::new(PluginSource::new_fake("test"));
    let other_source = Arc::new(PluginSource::new_fake("other"));
    let cv_ok = test_plugin_custom_value()
        .with_source(source.clone())
        .into_value(span);
    let cv_bad = test_plugin_custom_value()
        .with_source(other_source)
        .into_value(span);

    let fixtures = [
        (
            true, // should succeed
            PluginCall::Run(CallInfo {
                name: "".into(),
                call: EvaluatedCall {
                    head: span,
                    positional: vec![Value::test_int(4)],
                    named: vec![("x".to_owned().into_spanned(span), Some(Value::test_int(6)))],
                },
                input: PipelineData::empty(),
            }),
        ),
        (
            true, // should succeed
            PluginCall::Run(CallInfo {
                name: "".into(),
                call: EvaluatedCall {
                    head: span,
                    positional: vec![cv_ok.clone()],
                    named: vec![("ok".to_owned().into_spanned(span), Some(cv_ok.clone()))],
                },
                input: PipelineData::empty(),
            }),
        ),
        (
            false, // should fail
            PluginCall::Run(CallInfo {
                name: "".into(),
                call: EvaluatedCall {
                    head: span,
                    positional: vec![cv_bad.clone()],
                    named: vec![],
                },
                input: PipelineData::empty(),
            }),
        ),
        (
            false, // should fail
            PluginCall::Run(CallInfo {
                name: "".into(),
                call: EvaluatedCall {
                    head: span,
                    positional: vec![],
                    named: vec![("bad".to_owned().into_spanned(span), Some(cv_bad.clone()))],
                },
                input: PipelineData::empty(),
            }),
        ),
        (
            true, // should succeed
            PluginCall::Run(CallInfo {
                name: "".into(),
                call: EvaluatedCall {
                    head: span,
                    positional: vec![],
                    named: vec![],
                },
                // Shouldn't check input - that happens somewhere else
                input: PipelineData::value(cv_bad.clone(), None),
            }),
        ),
    ];

    for (should_succeed, mut fixture) in fixtures {
        let result = CurrentCallState::default().prepare_plugin_call(&mut fixture, &source);
        if should_succeed {
            assert!(
                result.is_ok(),
                "Expected success, but failed with {:?} on {fixture:#?}",
                result.unwrap_err(),
            );
        } else {
            assert!(
                result.is_err(),
                "Expected failure, but succeeded on {fixture:#?}",
            );
        }
    }
}

#[test]
fn prepare_plugin_call_custom_value_op() {
    // Check behavior with custom value ops
    let span = Span::test_data();
    let source = Arc::new(PluginSource::new_fake("test"));
    let other_source = Arc::new(PluginSource::new_fake("other"));
    let cv_ok = test_plugin_custom_value().with_source(source.clone());
    let cv_ok_val = cv_ok.clone_value(span);
    let cv_bad = test_plugin_custom_value().with_source(other_source);
    let cv_bad_val = cv_bad.clone_value(span);

    let fixtures = [
        (
            true, // should succeed
            PluginCall::CustomValueOp::<PipelineData>(
                Spanned {
                    item: cv_ok.clone().without_source(),
                    span,
                },
                CustomValueOp::ToBaseValue,
            ),
        ),
        (
            true, // should succeed
            PluginCall::CustomValueOp(
                Spanned {
                    item: test_plugin_custom_value(),
                    span,
                },
                // Dropped shouldn't check. We don't have a source set.
                CustomValueOp::Dropped,
            ),
        ),
        (
            true, // should succeed
            PluginCall::CustomValueOp::<PipelineData>(
                Spanned {
                    item: cv_ok.clone().without_source(),
                    span,
                },
                CustomValueOp::PartialCmp(cv_ok_val.clone()),
            ),
        ),
        (
            false, // should fail
            PluginCall::CustomValueOp(
                Spanned {
                    item: cv_ok.clone().without_source(),
                    span,
                },
                CustomValueOp::PartialCmp(cv_bad_val.clone()),
            ),
        ),
        (
            true, // should succeed
            PluginCall::CustomValueOp::<PipelineData>(
                Spanned {
                    item: cv_ok.clone().without_source(),
                    span,
                },
                CustomValueOp::Operation(
                    Operator::Math(Math::Concatenate).into_spanned(span),
                    cv_ok_val.clone(),
                ),
            ),
        ),
        (
            false, // should fail
            PluginCall::CustomValueOp(
                Spanned {
                    item: cv_ok.clone().without_source(),
                    span,
                },
                CustomValueOp::Operation(
                    Operator::Math(Math::Concatenate).into_spanned(span),
                    cv_bad_val.clone(),
                ),
            ),
        ),
    ];

    for (should_succeed, mut fixture) in fixtures {
        let result = CurrentCallState::default().prepare_plugin_call(&mut fixture, &source);
        if should_succeed {
            assert!(
                result.is_ok(),
                "Expected success, but failed with {:?} on {fixture:#?}",
                result.unwrap_err(),
            );
        } else {
            assert!(
                result.is_err(),
                "Expected failure, but succeeded on {fixture:#?}",
            );
        }
    }
}
