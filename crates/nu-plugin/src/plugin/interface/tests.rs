use crate::test_util::TestCaseExt;

use super::{EngineInterface, EngineInterfaceManager, ReceivedPluginCall};
use nu_engine::command_prelude::IoError;
use nu_plugin_core::{Interface, InterfaceManager, PluginWrite, interface_test_util::TestCase};
use nu_plugin_protocol::{
    ByteStreamInfo, CallInfo, CustomValueOp, EngineCall, EngineCallId, EngineCallResponse,
    EvaluatedCall, ListStreamInfo, PipelineDataHeader, PluginCall, PluginCallResponse,
    PluginCustomValue, PluginInput, PluginOption, PluginOutput, Protocol, ProtocolInfo, StreamData,
    test_util::{TestCustomValue, expected_test_custom_value, test_plugin_custom_value},
};
use nu_protocol::{
    BlockId, ByteStreamType, Config, CustomValue, DeclId, IntoInterruptiblePipelineData,
    LabeledError, PipelineData, PluginSignature, ShellError, Signals, Span, Spanned, Value, VarId,
    engine::Closure, shell_error,
};
use nu_utils::time::Instant;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::Ordering as AtomicOrdering,
        mpsc::{self, TryRecvError},
    },
    time::Duration,
};

const INPUT_TIMEOUT: Duration = Duration::from_secs(5);

/// Use the same Run dispatch and per-call interface that a plugin command receives.
fn receive_run(
    manager: &mut EngineInterfaceManager,
    id: usize,
    input: PipelineDataHeader,
) -> Result<(EngineInterface, PipelineData), ShellError> {
    manager.consume(PluginInput::Call(
        id,
        PluginCall::Run(CallInfo {
            name: "test-input".into(),
            call: EvaluatedCall::new(Span::test_data()),
            input,
        }),
    ))?;
    match manager
        .plugin_call_receiver
        .as_ref()
        .expect("missing receiver")
        .try_recv()
        .expect("Run was not dispatched")
    {
        ReceivedPluginCall::Run { engine, call } => Ok((engine, call.input)),
        other => panic!("expected Run, got {other:?}"),
    }
}

#[track_caller]
fn assert_interrupted(values: impl IntoIterator<Item = Value>) {
    let values = values.into_iter().collect::<Vec<_>>();
    assert!(matches!(values.as_slice(), [Value::Error { error, .. }]
        if matches!(error.as_ref(), ShellError::Interrupted { .. })));
}

#[test]
fn input_cleanup_ignores_value_empty_and_non_run_interfaces() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    set_default_protocol_info(&mut manager)?;
    assert!(manager.get_interface().input.is_none());
    manager.get_interface().finish_input()?;
    assert!(manager.interface_for_context(42).input.is_none());
    manager.interface_for_context(42).finish_input()?;
    for header in [
        PipelineDataHeader::Empty,
        PipelineDataHeader::value(Value::test_int(42)),
    ] {
        let (engine, _input) = receive_run(&mut manager, 0, header)?;
        assert!(engine.input.is_none());
        engine.finish_input()?;
    }
    Ok(())
}

#[test]
fn input_completion_isolates_calls_and_engine_call_results() -> Result<(), ShellError> {
    let test = TestCase::new();
    let mut manager = test.engine();
    set_default_protocol_info(&mut manager)?;
    let (engine_a, input_a) = receive_run(
        &mut manager,
        0,
        PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
    )?;
    let (engine_b, input_b) = receive_run(
        &mut manager,
        1,
        PipelineDataHeader::list_stream(ListStreamInfo::new(11, Span::test_data())),
    )?;
    assert!(engine_b.input.is_some());
    let (started, started_rx) = mpsc::channel();
    let (done, done_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = started.send(());
        let _ = done.send(input_a.into_iter().collect::<Vec<_>>());
    });
    started_rx
        .recv_timeout(INPUT_TIMEOUT)
        .expect("reader did not start");
    engine_a
        .write_response(Ok::<_, ShellError>(PipelineData::empty()))?
        .write()?;
    engine_a.clone().finish_input()?;
    let result = done_rx.recv_timeout(INPUT_TIMEOUT);
    if result.is_err() {
        manager.consume(PluginInput::End(10))?;
    }
    worker.join().expect("reader panicked");
    assert_interrupted(result.expect("input A did not stop"));
    assert!(!engine_b.signals().interrupted());
    manager.consume(PluginInput::Data(11, Value::test_int(42).into()))?;
    manager.consume(PluginInput::End(11))?;
    assert_eq!(
        input_b.into_iter().collect::<Vec<_>>(),
        vec![Value::test_int(42)]
    );
    // An engine call result uses another reader, even in A's cancelled context.
    let response_rx = fake_engine_call(&mut manager, 0);
    manager.consume(PluginInput::EngineCallResponse(
        0,
        EngineCallResponse::PipelineData(PipelineDataHeader::list_stream(ListStreamInfo::new(
            12,
            Span::test_data(),
        ))),
    ))?;
    manager.consume(PluginInput::Data(12, Value::test_int(43).into()))?;
    manager.consume(PluginInput::End(12))?;
    let EngineCallResponse::PipelineData(result) =
        response_rx.try_recv().expect("missing response")
    else {
        panic!("expected pipeline data");
    };
    assert_eq!(
        result.into_iter().collect::<Vec<_>>(),
        vec![Value::test_int(43)]
    );
    // Late messages for A are still legal until the producer sends its actual End.
    manager.consume(PluginInput::Data(10, Value::test_int(99).into()))?;
    manager.consume(PluginInput::End(10))?;
    let (engine_c, input_c) = receive_run(
        &mut manager,
        2,
        PipelineDataHeader::value(Value::test_int(44)),
    )?;
    assert!(engine_c.input.is_none());
    assert_eq!(input_c.into_value(Span::test_data())?, Value::test_int(44));
    assert_eq!(
        test.written()
            .filter(|msg| matches!(msg, PluginOutput::Drop(10)))
            .count(),
        1
    );
    Ok(())
}

#[test]
fn input_completion_wakes_list_and_byte_streams_after_run_response() -> Result<(), ShellError> {
    for header in [
        PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
        PipelineDataHeader::byte_stream(ByteStreamInfo::new(
            10,
            Span::test_data(),
            ByteStreamType::Binary,
        )),
    ] {
        let mut manager = TestCase::new().engine();
        set_default_protocol_info(&mut manager)?;
        let (engine, input) = receive_run(&mut manager, 0, header)?;
        engine
            .write_response(Ok::<_, ShellError>(PipelineData::empty()))?
            .write()?;
        let (done, done_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let result = match input {
                PipelineData::ListStream(stream, _) => match stream.into_iter().next() {
                    Some(Value::Error { error, .. }) => Err(*error),
                    value => Ok(value.is_none()),
                },
                PipelineData::ByteStream(stream, _) => {
                    stream.into_bytes().map(|bytes| bytes.is_empty())
                }
                other => panic!("expected stream, got {other:?}"),
            };
            let _ = done.send(result);
        });
        engine.finish_input()?;
        drop(engine);
        let result = done_rx.recv_timeout(INPUT_TIMEOUT);
        if result.is_err() {
            manager.consume(PluginInput::End(10))?;
        }
        worker.join().expect("reader panicked");
        assert!(matches!(
            result.expect("reader did not stop"),
            Err(ShellError::Interrupted { .. })
        ));
    }
    Ok(())
}

#[test]
fn input_completion_does_not_retract_buffered_bytes() -> Result<(), ShellError> {
    use std::io::Read;

    let mut manager = TestCase::new().engine();
    set_default_protocol_info(&mut manager)?;
    let (engine, input) = receive_run(
        &mut manager,
        0,
        PipelineDataHeader::byte_stream(ByteStreamInfo::new(
            10,
            Span::test_data(),
            ByteStreamType::Binary,
        )),
    )?;
    manager.consume(PluginInput::Data(10, StreamData::Raw(Ok(vec![1, 2]))))?;
    let PipelineData::ByteStream(stream, _) = input else {
        panic!("expected byte stream");
    };
    let mut reader = stream.reader().expect("missing byte reader");
    let mut byte = [0];
    assert_eq!(reader.read(&mut byte).expect("read failed"), 1);
    assert_eq!(byte, [1]);
    engine.finish_input()?;
    // This byte has already left the transport queue and belongs to the byte reader's buffer.
    assert_eq!(reader.read(&mut byte).expect("read failed"), 1);
    assert_eq!(byte, [2]);
    let (done, done_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = done.send(reader.read(&mut byte));
    });
    let result = done_rx.recv_timeout(INPUT_TIMEOUT);
    if result.is_err() {
        manager.consume(PluginInput::End(10))?;
    }
    worker.join().expect("reader panicked");
    let error = result
        .expect("reader waited for new transport bytes")
        .expect_err("cancellation looked like EOF");
    let nu_protocol::shell_error::bridge::ShellErrorBridge(error) =
        nu_protocol::shell_error::bridge::ShellErrorBridge::try_from(error)
            .expect("missing shell error");
    assert!(matches!(error, ShellError::Interrupted { .. }));
    Ok(())
}

#[test]
fn input_completion_uses_plugin_wide_gc_option_at_completion() -> Result<(), ShellError> {
    for disabled in [false, true] {
        let test = TestCase::new();
        let mut manager = test.engine();
        set_default_protocol_info(&mut manager)?;
        let (engine, input) = receive_run(
            &mut manager,
            0,
            PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
        )?;
        // GC is process-wide, and changes after Run dispatch must still affect its cleanup.
        let global = manager.get_interface();
        global.set_gc_disabled(true)?;
        global.set_gc_disabled(disabled)?;
        engine.finish_input()?;
        assert!(!engine.signals().interrupted());
        manager.consume(PluginInput::Data(10, Value::test_int(42).into()))?;
        manager.consume(PluginInput::End(10))?;
        if disabled {
            assert_eq!(
                input.into_iter().collect::<Vec<_>>(),
                vec![Value::test_int(42)]
            );
        } else {
            assert_interrupted(input);
        }
        assert!(test.written().any(|msg| matches!(
            msg,
            PluginOutput::Option(PluginOption::GcDisabled(value)) if value == disabled
        )));
    }
    Ok(())
}

#[test]
fn input_completion_waits_for_all_sdk_forwarders() -> Result<(), ShellError> {
    for disabled in [false, true] {
        let mut manager = TestCase::new().engine();
        set_default_protocol_info(&mut manager)?;
        let (engine, input) = receive_run(
            &mut manager,
            0,
            PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
        )?;
        engine.set_gc_disabled(disabled)?;
        let call_input = engine.input.as_ref().expect("missing call input");
        let first = call_input.begin_forwarding()?;
        let last = call_input.begin_forwarding()?;
        engine.finish_input()?;
        drop(first);
        manager.consume(PluginInput::Data(10, Value::test_int(42).into()))?;
        let mut input = input.into_iter();
        assert_eq!(input.next(), Some(Value::test_int(42)));
        drop(last);
        assert_eq!(call_input.lock()?.active, 0);
        // Real End also bounds the receive if deferred cleanup regresses.
        manager.consume(PluginInput::End(10))?;
        if disabled {
            assert!(input.next().is_none());
        } else {
            assert_interrupted(input);
        }
    }
    Ok(())
}

#[test]
fn input_completion_does_not_truncate_call_decl_forwarding() -> Result<(), ShellError> {
    let test = TestCase::new();
    let mut manager = test.engine();
    set_default_protocol_info(&mut manager)?;
    let (engine, input) = receive_run(
        &mut manager,
        0,
        PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
    )?;
    let caller = engine.clone();
    let (done, done_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = caller.call_decl(
            DeclId::new(42),
            EvaluatedCall::new(Span::test_data()),
            input,
            false,
            false,
        );
        let _ = done.send(result);
    });
    let (id, subscription) = manager
        .engine_call_subscription_receiver
        .recv_timeout(INPUT_TIMEOUT)
        .expect("engine call was not registered");
    manager.engine_call_subscriptions.insert(id, subscription);
    // Like a non-redirected external, the engine responds without consuming the forwarded input.
    manager.consume(PluginInput::EngineCallResponse(
        id,
        EngineCallResponse::PipelineData(PipelineDataHeader::Empty),
    ))?;
    let response = done_rx
        .recv_timeout(INPUT_TIMEOUT)
        .expect("engine call did not return")?;
    worker.join().expect("caller panicked");
    engine
        .write_response(Ok::<_, ShellError>(response))?
        .write()?;
    // Establish completion synchronously, rather than depending on the peer's response timing.
    engine.finish_input()?;
    assert_eq!(
        engine
            .input
            .as_ref()
            .expect("missing call input")
            .lock()?
            .active,
        1
    );
    for n in 1..=5 {
        manager.consume(PluginInput::Data(10, Value::test_int(n).into()))?;
    }
    manager.consume(PluginInput::End(10))?;
    let started = Instant::now();
    let mut values = Vec::new();
    loop {
        match test.next_written() {
            Some(PluginOutput::Data(_, StreamData::List(value))) => values.push(value),
            Some(PluginOutput::End(_)) => break,
            Some(_) => (),
            None => {
                assert!(
                    started.elapsed() < INPUT_TIMEOUT,
                    "forwarder did not finish"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    assert_eq!(values, (1..=5).map(Value::test_int).collect::<Vec<_>>());
    assert_eq!(
        engine
            .input
            .as_ref()
            .expect("missing call input")
            .lock()?
            .active,
        0
    );
    Ok(())
}

#[test]
fn input_completion_races_last_sdk_forwarder() -> Result<(), ShellError> {
    for _ in 0..64 {
        let mut manager = TestCase::new().engine();
        set_default_protocol_info(&mut manager)?;
        let (engine, input) = receive_run(
            &mut manager,
            0,
            PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
        )?;
        let guard = engine
            .input
            .as_ref()
            .expect("missing call input")
            .begin_forwarding()?;
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let worker_barrier = barrier.clone();
        let worker = std::thread::spawn(move || {
            worker_barrier.wait();
            drop(guard);
        });
        barrier.wait();
        engine.finish_input()?;
        worker.join().expect("forwarder panicked");
        manager.consume(PluginInput::End(10))?;
        assert_interrupted(input);
    }
    Ok(())
}

#[test]
fn input_output_drop_rolls_back_instead_of_committing_prefix() -> Result<(), ShellError> {
    for byte_stream in [false, true] {
        for early_drop in [false, true] {
            let test = TestCase::new();
            let mut manager = test.engine();
            set_default_protocol_info(&mut manager)?;
            let header = if byte_stream {
                PipelineDataHeader::byte_stream(ByteStreamInfo::new(
                    10,
                    Span::test_data(),
                    ByteStreamType::Binary,
                ))
            } else {
                PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data()))
            };
            let (engine, input) = receive_run(&mut manager, 0, header)?;
            engine.set_gc_disabled(true)?;
            let output =
                std::iter::empty::<Value>().into_pipeline_data(Span::test_data(), Signals::empty());
            let writer = engine.write_response(Ok::<_, ShellError>(output))?;
            let output_id = test
                .written()
                .find_map(|message| match message {
                    PluginOutput::CallResponse(0, PluginCallResponse::PipelineData(header)) => {
                        header.stream_id()
                    }
                    _ => None,
                })
                .expect("missing output id");
            let (read, read_rx) = mpsc::channel();
            let (done, done_rx) = mpsc::channel();
            let worker = std::thread::spawn(move || {
                let result = match input {
                    PipelineData::ListStream(stream, _) => {
                        stream.into_iter().try_for_each(|value| {
                            if let Value::Error { error, .. } = value {
                                Err(*error)
                            } else {
                                let _ = read.send(());
                                Ok(())
                            }
                        })
                    }
                    PipelineData::ByteStream(stream, _) => {
                        use nu_protocol::shell_error::bridge::ShellErrorBridge;
                        use std::io::Read;
                        let mut reader = stream.reader().expect("missing reader");
                        let mut buffer = [0; 16];
                        loop {
                            match reader.read(&mut buffer) {
                                Ok(0) => break Ok(()),
                                Ok(_) => {
                                    let _ = read.send(());
                                }
                                Err(error) => {
                                    break Err(ShellErrorBridge::try_from(error)
                                        .expect("missing shell error")
                                        .0);
                                }
                            }
                        }
                    }
                    other => panic!("expected stream, got {other:?}"),
                };
                // A transaction commits only on successful EOF, and rolls back on interruption.
                let committed = result.is_ok();
                let _ = done.send((result, committed));
            });
            let data = if byte_stream {
                StreamData::Raw(Ok(vec![1, 2]))
            } else {
                Value::test_int(42).into()
            };
            manager.consume(PluginInput::Data(10, data))?;
            read_rx
                .recv_timeout(INPUT_TIMEOUT)
                .expect("consumer did not read the prefix");
            if early_drop {
                manager.consume(PluginInput::Drop(output_id))?;
            } else {
                manager.consume(PluginInput::End(10))?;
            }
            let result = done_rx.recv_timeout(INPUT_TIMEOUT);
            if early_drop {
                manager.consume(PluginInput::End(10))?;
            }
            worker.join().expect("consumer panicked");
            let (result, committed) = result.expect("consumer did not stop");
            assert_eq!(committed, !early_drop);
            if early_drop {
                assert!(matches!(result, Err(ShellError::Interrupted { .. })));
            } else {
                result?;
                manager.consume(PluginInput::Drop(output_id))?;
            }
            writer.write()?;
            engine.finish_input()?;
            assert!(
                !test
                    .written()
                    .any(|message| matches!(message, PluginOutput::Data(..)))
            );
        }
    }
    Ok(())
}

#[test]
fn input_cleanup_does_not_keep_engine_interface_alive() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    set_default_protocol_info(&mut manager)?;
    let (engine, input) = receive_run(
        &mut manager,
        0,
        PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
    )?;
    let weak = Arc::downgrade(&engine.state);
    let cancellation = engine
        .input
        .as_ref()
        .expect("missing call input")
        .cancellation
        .clone();
    drop(input);
    drop(engine);
    drop(manager);
    assert!(weak.upgrade().is_none());
    cancellation.cancel();
    Ok(())
}

#[test]
fn input_gc_option_write_failure_does_not_disable_cleanup() -> Result<(), ShellError> {
    let test = TestCase::new();
    let mut manager = test.engine();
    set_default_protocol_info(&mut manager)?;
    let (engine, input) = receive_run(
        &mut manager,
        0,
        PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
    )?;
    test.set_write_error(ShellError::NushellFailed {
        msg: "write failed".into(),
    });
    assert!(engine.set_gc_disabled(true).is_err());
    engine.finish_input()?;
    manager.consume(PluginInput::Data(10, Value::test_int(42).into()))?;
    manager.consume(PluginInput::End(10))?;
    assert_interrupted(input);
    Ok(())
}

#[test]
fn input_gc_option_matches_wire_order_for_concurrent_calls() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = test.engine();
    let interface = manager.get_interface();
    std::thread::scope(|scope| {
        let workers = (0..8)
            .map(|index| {
                let interface = interface.clone();
                scope.spawn(move || {
                    for change in 0..16 {
                        interface.set_gc_disabled((index + change) % 2 == 0)?;
                    }
                    Ok::<_, ShellError>(())
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().expect("GC option writer panicked")?;
        }
        Ok::<_, ShellError>(())
    })?;
    let last = test
        .written()
        .filter_map(|msg| match msg {
            PluginOutput::Option(PluginOption::GcDisabled(disabled)) => Some(disabled),
            _ => None,
        })
        .last()
        .expect("no GC option was sent");
    assert_eq!(
        interface.state.gc_disabled.load(AtomicOrdering::Acquire),
        last
    );
    Ok(())
}

#[test]
fn input_gc_option_flush_failure_preserves_last_successful_setting() -> Result<(), ShellError> {
    for previously_disabled in [false, true] {
        let test = TestCase::new();
        let mut manager = test.engine();
        set_default_protocol_info(&mut manager)?;
        let (engine, input) = receive_run(
            &mut manager,
            0,
            PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
        )?;
        engine.set_gc_disabled(previously_disabled)?;
        test.set_flush_error(ShellError::NushellFailed {
            msg: "flush failed".into(),
        });
        assert!(matches!(
            engine.set_gc_disabled(!previously_disabled),
            Err(ShellError::NushellFailed { msg }) if msg == "flush failed"
        ));
        assert!(!test.was_flushed());
        engine.finish_input()?;
        manager.consume(PluginInput::Data(10, Value::test_int(42).into()))?;
        manager.consume(PluginInput::End(10))?;
        if previously_disabled {
            assert_eq!(
                input.into_iter().collect::<Vec<_>>(),
                vec![Value::test_int(42)]
            );
        } else {
            assert_interrupted(input);
        }
    }
    Ok(())
}

#[test]
fn input_completion_does_not_wait_for_gc_option_write_or_flush() -> Result<(), ShellError> {
    struct BlockingWriter {
        test: TestCase<PluginInput, PluginOutput>,
        block_flush: bool,
        waiting: mpsc::Sender<()>,
        resume: Mutex<Option<mpsc::Receiver<()>>>,
    }

    impl BlockingWriter {
        fn pause(&self) {
            let resume = self.resume.lock().expect("GC gate poisoned").take();
            if let Some(resume) = resume {
                self.waiting.send(()).expect("GC observer disconnected");
                resume
                    .recv_timeout(INPUT_TIMEOUT)
                    .expect("GC write was not released");
            }
        }
    }

    impl PluginWrite<PluginOutput> for BlockingWriter {
        fn write(&self, data: &PluginOutput) -> Result<(), ShellError> {
            if !self.block_flush && matches!(data, PluginOutput::Option(_)) {
                self.pause();
            }
            self.test.write(data)
        }

        fn flush(&self) -> Result<(), ShellError> {
            if self.block_flush {
                self.pause();
            }
            self.test.flush()
        }
    }

    for block_flush in [false, true] {
        let (waiting, waiting_rx) = mpsc::channel();
        let (resume, resume_rx) = mpsc::channel();
        let mut manager = EngineInterfaceManager::new(BlockingWriter {
            test: TestCase::new(),
            block_flush,
            waiting,
            resume: Mutex::new(Some(resume_rx)),
        });
        set_default_protocol_info(&mut manager)?;
        let (engine, input) = receive_run(
            &mut manager,
            0,
            PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
        )?;
        let global = manager.get_interface();
        let option_writer = std::thread::spawn(move || global.set_gc_disabled(true));
        waiting_rx
            .recv_timeout(INPUT_TIMEOUT)
            .expect("GC write did not block");
        let (done, done_rx) = mpsc::channel();
        let cleanup = std::thread::spawn(move || {
            let _ = done.send(engine.finish_input());
        });
        let result = done_rx.recv_timeout(INPUT_TIMEOUT);
        // Release both workers before asserting, even if cleanup still waits for the I/O lock.
        let _ = resume.send(());
        option_writer.join().expect("GC option writer panicked")?;
        cleanup.join().expect("input cleanup panicked");
        result.expect("input completion waited for unrelated transport I/O")?;
        manager.consume(PluginInput::End(10))?;
        assert_interrupted(input);
    }
    Ok(())
}

#[test]
fn input_failed_response_header_still_cleans_up_input() -> Result<(), ShellError> {
    let test = TestCase::new();
    let mut manager = test.engine();
    set_default_protocol_info(&mut manager)?;
    let (engine, input) = receive_run(
        &mut manager,
        0,
        PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
    )?;
    let output =
        std::iter::empty::<Value>().into_pipeline_data(Span::test_data(), Signals::empty());
    test.set_write_error(ShellError::NushellFailed {
        msg: "response write failed".into(),
    });
    let error = engine
        .write_response(Ok::<_, ShellError>(output))
        .err()
        .expect("header write succeeded");
    assert!(matches!(error, ShellError::NushellFailed { msg } if msg == "response write failed"));
    engine.finish_input()?;
    manager.consume(PluginInput::Data(10, Value::test_int(42).into()))?;
    manager.consume(PluginInput::End(10))?;
    assert_interrupted(input);
    Ok(())
}

#[test]
fn input_output_drop_ends_input_even_with_gc_disabled() -> Result<(), ShellError> {
    for disabled in [false, true] {
        let test = TestCase::new();
        let mut manager = test.engine();
        set_default_protocol_info(&mut manager)?;
        let (engine, input) = receive_run(
            &mut manager,
            0,
            PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
        )?;
        engine.set_gc_disabled(disabled)?;
        let output =
            std::iter::empty::<Value>().into_pipeline_data(Span::test_data(), Signals::empty());
        let writer = engine.write_response(Ok::<_, ShellError>(output))?;
        let output_id = test
            .written()
            .find_map(|message| match message {
                PluginOutput::CallResponse(0, PluginCallResponse::PipelineData(header)) => {
                    header.stream_id()
                }
                _ => None,
            })
            .expect("missing output id");
        let (done, done_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _ = done.send(input.into_iter().next());
        });
        manager.consume(PluginInput::Drop(output_id))?;
        manager.consume(PluginInput::Drop(output_id))?;
        let result = done_rx.recv_timeout(INPUT_TIMEOUT);
        if result.is_err() {
            manager.consume(PluginInput::End(10))?;
        }
        worker.join().expect("reader panicked");
        assert_interrupted(result.expect("output Drop did not end input"));
        writer.write()?;
        engine.finish_input()?;
        assert_eq!(
            test.written()
                .filter(|msg| matches!(msg, PluginOutput::Drop(10)))
                .count(),
            1
        );
    }
    Ok(())
}

#[test]
fn input_output_association_is_removed_after_completion() -> Result<(), ShellError> {
    for disabled in [false, true] {
        let test = TestCase::new();
        let mut manager = test.engine();
        set_default_protocol_info(&mut manager)?;
        let (engine, input) = receive_run(
            &mut manager,
            0,
            PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
        )?;
        engine.set_gc_disabled(disabled)?;
        let output =
            std::iter::empty::<Value>().into_pipeline_data(Span::test_data(), Signals::empty());
        let writer = engine.write_response(Ok::<_, ShellError>(output))?;
        let output_id = test
            .written()
            .find_map(|message| match message {
                PluginOutput::CallResponse(0, PluginCallResponse::PipelineData(header)) => {
                    header.stream_id()
                }
                _ => None,
            })
            .expect("missing output id");
        writer.write()?;
        // A Drop acknowledging natural End can arrive before the runner's completion hook.
        manager.consume(PluginInput::Drop(output_id))?;
        engine.finish_input()?;
        // A late Drop must not revoke the GC opt-out after the response already completed.
        manager.consume(PluginInput::Drop(output_id))?;
        manager.consume(PluginInput::Data(10, Value::test_int(42).into()))?;
        manager.consume(PluginInput::End(10))?;
        if disabled {
            assert_eq!(
                input.into_iter().collect::<Vec<_>>(),
                vec![Value::test_int(42)]
            );
        } else {
            assert_interrupted(input);
        }
    }
    Ok(())
}

#[test]
fn input_engine_call_stream_drop_does_not_cancel_original_input() -> Result<(), ShellError> {
    let mut manager = TestCase::new().engine();
    set_default_protocol_info(&mut manager)?;
    let (engine, input) = receive_run(
        &mut manager,
        0,
        PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data())),
    )?;
    let data = std::iter::empty::<Value>().into_pipeline_data(Span::test_data(), Signals::empty());
    let (header, writer) = engine.init_write_pipeline_data(data, &())?;
    manager.consume(PluginInput::Drop(
        header.stream_id().expect("missing stream id"),
    ))?;
    manager.consume(PluginInput::Data(10, Value::test_int(42).into()))?;
    manager.consume(PluginInput::End(10))?;
    assert_eq!(
        input.into_iter().collect::<Vec<_>>(),
        vec![Value::test_int(42)]
    );
    writer.write()?;
    engine.finish_input()?;
    Ok(())
}

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
    let error: ShellError = LabeledError::new("this is an error")
        .with_help("a test error")
        .into();
    interface.write_response(Err(error.clone()))?.write()?;

    let written = test.next_written().expect("nothing written");

    match written {
        PluginOutput::CallResponse(id, response) => {
            assert_eq!(35, id, "id");
            match response {
                PluginCallResponse::Error(err) => assert_eq!(error, err),
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
