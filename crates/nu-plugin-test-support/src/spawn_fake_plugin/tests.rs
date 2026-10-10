use super::{FakePluginRead, FakePluginWrite, fake_plugin_channel};
use nu_plugin::{EngineInterface, EvaluatedCall, Plugin, PluginCommand};
use nu_plugin_core::PluginWrite;
use nu_plugin_protocol::{
    ByteStreamInfo, CallInfo, EngineCall, EngineCallResponse, ListStreamInfo, PipelineDataHeader,
    PluginCall, PluginCallResponse, PluginInput, PluginOption, PluginOutput, ProtocolInfo,
    StreamData,
};
use nu_protocol::{
    BlockId, ByteStreamType, DeclId, IntoInterruptiblePipelineData, LabeledError, PipelineData,
    ShellError, Signals, Signature, Span, Spanned, Value, engine::Closure,
};
use std::{sync::mpsc, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(5);

/// A real SDK serve loop with an in-memory engine peer and bounded shutdown.
struct SdkTest {
    peer: FakePluginWrite<PluginInput>,
    output: FakePluginRead<PluginOutput>,
    done: mpsc::Receiver<Result<(), String>>,
    runner: Option<std::thread::JoinHandle<()>>,
}

impl SdkTest {
    fn new(plugin: InputPlugin) -> Result<Self, ShellError> {
        let (input, peer) = fake_plugin_channel::<PluginInput>();
        let (output, writer) = fake_plugin_channel::<PluginOutput>();
        let (done, done_rx) = mpsc::channel();
        let runner = std::thread::spawn(move || {
            let result =
                nu_plugin::serve_plugin_io(&plugin, "input-cleanup", move || input, move || writer);
            let _ = done.send(result.map_err(|error| error.to_string()));
        });
        let test = Self {
            peer,
            output,
            done: done_rx,
            runner: Some(runner),
        };
        test.peer
            .write(&PluginInput::Hello(ProtocolInfo::default()))?;
        assert!(matches!(
            test.output.0.recv_timeout(TIMEOUT),
            Ok(PluginOutput::Hello(_))
        ));
        Ok(test)
    }

    fn run(&self, call_id: usize, name: &str, byte_stream: bool) -> Result<(), ShellError> {
        let input_id = 10 + call_id;
        let input = if byte_stream {
            PipelineDataHeader::byte_stream(ByteStreamInfo::new(
                input_id,
                Span::test_data(),
                ByteStreamType::Binary,
            ))
        } else {
            PipelineDataHeader::list_stream(ListStreamInfo::new(input_id, Span::test_data()))
        };
        self.peer.write(&PluginInput::Call(
            call_id,
            PluginCall::Run(CallInfo {
                name: name.into(),
                call: EvaluatedCall::new(Span::test_data()),
                input,
            }),
        ))
    }

    /// Close call intake and join every SDK runner without disconnecting transport input.
    /// Unlike a response header or End, this proves that each response's cleanup has finished.
    fn finish_calls(&mut self) -> Result<(), ShellError> {
        if let Some(runner) = self.runner.take() {
            self.peer.write(&PluginInput::Goodbye)?;
            self.done
                .recv_timeout(TIMEOUT)
                .expect("SDK runners did not finish")
                .expect("SDK failed");
            runner.join().expect("SDK runner panicked");
        }
        Ok(())
    }

    fn stop(mut self) -> Result<(), ShellError> {
        self.finish_calls()?;
        drop(self.peer);
        Ok(())
    }
}

fn input_data(byte_stream: bool) -> StreamData {
    if byte_stream {
        StreamData::Raw(Ok(vec![1, 2]))
    } else {
        Value::test_int(42).into()
    }
}

struct InputPlugin {
    gc_disabled: bool,
    background: Option<mpsc::Sender<BackgroundReader>>,
}
struct Passthrough;
struct BackgroundInput(&'static str);
struct ForwardInput(bool);

struct BackgroundReader {
    started: mpsc::Receiver<()>,
    result: mpsc::Receiver<Result<bool, ShellError>>,
    thread: std::thread::JoinHandle<()>,
}

impl BackgroundReader {
    /// With no data, record the result while upstream is still idle, before sending teardown End.
    /// Otherwise consume real data and End to prove that the reader survived completion.
    fn finish(
        self,
        peer: &FakePluginWrite<PluginInput>,
        data: Option<StreamData>,
    ) -> Result<bool, ShellError> {
        let upstream_idle = data.is_none();
        if let Some(data) = data {
            peer.write(&PluginInput::Data(10, data))?;
            peer.write(&PluginInput::End(10))?;
        }
        let result = self.result.recv_timeout(TIMEOUT);
        if upstream_idle {
            peer.write(&PluginInput::End(10))?;
        }
        self.thread.join().expect("input reader panicked");
        result.expect("input reader did not finish")
    }
}

impl Plugin for InputPlugin {
    fn version(&self) -> String {
        "0.0.0".into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(Passthrough),
            Box::new(BackgroundInput("background")),
            Box::new(BackgroundInput("background-error")),
            Box::new(BackgroundInput("background-stream")),
            Box::new(BackgroundInput("background-output")),
            Box::new(ForwardInput(false)),
            Box::new(ForwardInput(true)),
        ]
    }
}

impl PluginCommand for Passthrough {
    type Plugin = InputPlugin;

    fn name(&self) -> &str {
        "test-input"
    }

    fn description(&self) -> &str {
        "Return a lazy input stream for testing cancellation"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
    }

    fn run(
        &self,
        plugin: &InputPlugin,
        engine: &EngineInterface,
        _call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        if plugin.gc_disabled {
            engine.set_gc_disabled(true)?;
        }
        Ok(input)
    }
}

impl PluginCommand for BackgroundInput {
    type Plugin = InputPlugin;

    fn name(&self) -> &str {
        self.0
    }

    fn description(&self) -> &str {
        "Leave input on a background thread after returning a response"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
    }

    fn run(
        &self,
        plugin: &InputPlugin,
        engine: &EngineInterface,
        _call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        if plugin.gc_disabled {
            engine.set_gc_disabled(true)?;
        }
        let (started_tx, started) = mpsc::channel();
        let (done, result) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let _ = started_tx.send(());
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
        plugin
            .background
            .as_ref()
            .expect("missing background reader channel")
            .send(BackgroundReader {
                started,
                result,
                thread,
            })
            .expect("background reader receiver closed");
        if self.0 == "background-error" {
            Err(LabeledError::new("background input failure"))
        } else if self.0 == "background-stream" {
            Ok(std::iter::empty::<Value>().into_pipeline_data(Span::test_data(), Signals::empty()))
        } else if self.0 == "background-output" {
            Ok((0..200)
                .map(Value::test_int)
                .into_pipeline_data(Span::test_data(), Signals::empty()))
        } else {
            Ok(PipelineData::empty())
        }
    }
}

impl PluginCommand for ForwardInput {
    type Plugin = InputPlugin;

    fn name(&self) -> &str {
        if self.0 {
            "forward-closure"
        } else {
            "forward-decl"
        }
    }

    fn description(&self) -> &str {
        "Forward input to an engine call that responds before consuming the stream"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
    }

    fn run(
        &self,
        _plugin: &InputPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        if self.0 {
            Ok(engine.eval_closure_with_stream(
                &Spanned {
                    item: Closure {
                        block_id: BlockId::new(42),
                        captures: vec![],
                    },
                    span: call.head,
                },
                vec![],
                input,
                false,
                false,
            )?)
        } else {
            Ok(engine.call_decl(
                DeclId::new(42),
                EvaluatedCall::new(call.head),
                input,
                false,
                false,
            )?)
        }
    }
}

/// A non-redirected engine call can respond Empty while its SDK-owned input writer is still idle.
/// Completing the plugin response must preserve that writer, but dropping its stream must release
/// deferred cleanup without waiting for the upstream producer to send data.
#[test]
fn sdk_completion_preserves_engine_call_forwarding_until_consumed_or_dropped()
-> Result<(), ShellError> {
    for name in ["forward-decl", "forward-closure"] {
        for byte_stream in [false, true] {
            for drop_forwarded_input in [false, true] {
                let sdk = SdkTest::new(InputPlugin {
                    gc_disabled: false,
                    background: None,
                })?;
                let peer = &sdk.peer;
                let output = &sdk.output;
                sdk.run(0, name, byte_stream)?;
                let (engine_call_id, forwarded_id) =
                    match output.0.recv_timeout(TIMEOUT).expect("missing engine call") {
                        PluginOutput::EngineCall {
                            id,
                            call:
                                EngineCall::CallDecl {
                                    input,
                                    redirect_stdout: false,
                                    ..
                                }
                                | EngineCall::EvalClosure {
                                    input,
                                    redirect_stdout: false,
                                    ..
                                },
                            ..
                        } => (id, input.stream_id().expect("missing forwarded stream")),
                        other => panic!("unexpected output: {other:?}"),
                    };
                peer.write(&PluginInput::EngineCallResponse(
                    engine_call_id,
                    EngineCallResponse::PipelineData(PipelineDataHeader::Empty),
                ))?;
                assert!(matches!(
                    output.0.recv_timeout(TIMEOUT),
                    Ok(PluginOutput::CallResponse(
                        0,
                        PluginCallResponse::PipelineData(PipelineDataHeader::Empty)
                    ))
                ));
                if drop_forwarded_input {
                    peer.write(&PluginInput::Drop(forwarded_id))?;
                } else {
                    // Only start producing input after the plugin's own response has completed.
                    for n in 1..=5 {
                        let data = if byte_stream {
                            StreamData::Raw(Ok(vec![n]))
                        } else {
                            Value::test_int(n.into()).into()
                        };
                        peer.write(&PluginInput::Data(10, data))?;
                    }
                    peer.write(&PluginInput::End(10))?;
                }
                let mut forwarded_values = Vec::new();
                let mut forwarded_bytes = Vec::new();
                let mut input_drops = 0;
                let mut output_ended = false;
                while input_drops == 0 || !output_ended {
                    match output
                        .0
                        .recv_timeout(TIMEOUT)
                        .expect("forwarder did not finish")
                    {
                        PluginOutput::Ack(10) if !drop_forwarded_input => (),
                        PluginOutput::Data(id, StreamData::List(value)) if id == forwarded_id => {
                            forwarded_values.push(value)
                        }
                        PluginOutput::Data(id, StreamData::Raw(bytes)) if id == forwarded_id => {
                            forwarded_bytes.extend(bytes?)
                        }
                        PluginOutput::Drop(10) => input_drops += 1,
                        PluginOutput::End(id) if id == forwarded_id => output_ended = true,
                        other => panic!("unexpected forwarding output: {other:?}"),
                    }
                }
                if drop_forwarded_input {
                    assert!(forwarded_values.is_empty() && forwarded_bytes.is_empty());
                    let late_data = if byte_stream {
                        StreamData::Raw(Ok(vec![99]))
                    } else {
                        Value::test_int(99).into()
                    };
                    peer.write(&PluginInput::Data(10, late_data))?;
                    peer.write(&PluginInput::End(10))?;
                } else if byte_stream {
                    assert_eq!(forwarded_bytes, vec![1, 2, 3, 4, 5]);
                    assert!(forwarded_values.is_empty());
                } else {
                    assert_eq!(
                        forwarded_values,
                        (1..=5).map(Value::test_int).collect::<Vec<_>>()
                    );
                    assert!(forwarded_bytes.is_empty());
                }
                assert_eq!(input_drops, 1);
                sdk.stop()?;
            }
        }
    }
    Ok(())
}

#[test]
fn sdk_output_drop_cancels_input_before_flow_control_writer_finishes_with_gc_disabled()
-> Result<(), ShellError> {
    for _ in 0..32 {
        let (background, background_rx) = mpsc::channel();
        let sdk = SdkTest::new(InputPlugin {
            gc_disabled: true,
            background: Some(background),
        })?;
        let peer = &sdk.peer;
        let output = &sdk.output;
        sdk.run(0, "background-output", false)?;
        let reader = background_rx
            .recv_timeout(TIMEOUT)
            .expect("missing input reader");
        reader
            .started
            .recv_timeout(TIMEOUT)
            .expect("reader did not start");
        assert!(matches!(
            output.0.recv_timeout(TIMEOUT),
            Ok(PluginOutput::Option(PluginOption::GcDisabled(true)))
        ));
        let output_id = match output.0.recv_timeout(TIMEOUT).expect("missing response") {
            PluginOutput::CallResponse(0, PluginCallResponse::PipelineData(header)) => {
                header.stream_id().expect("missing output id")
            }
            other => panic!("unexpected response: {other:?}"),
        };
        // No Ack: writing the 100th list item reaches the SDK's flow-control threshold.
        for n in 0..100 {
            assert!(matches!(output.0.recv_timeout(TIMEOUT),
                Ok(PluginOutput::Data(id, StreamData::List(value))) if id == output_id && value == Value::test_int(n)));
        }
        peer.write(&PluginInput::Drop(output_id))?;
        assert!(matches!(
            reader.finish(peer, None),
            Err(ShellError::Interrupted { .. })
        ));
        let mut input_drops = 0;
        let mut output_ended = false;
        while input_drops == 0 || !output_ended {
            match output
                .0
                .recv_timeout(TIMEOUT)
                .expect("writer did not finish")
            {
                PluginOutput::Drop(10) => input_drops += 1,
                PluginOutput::End(id) if id == output_id => output_ended = true,
                other => panic!("unexpected output after Drop: {other:?}"),
            }
        }
        assert_eq!(input_drops, 1);
        sdk.stop()?;
    }
    Ok(())
}

/// Drive the real SDK's serve loop with the same in-memory transport as PluginTest. Both inputs
/// stay open and idle until A's output is dropped; B receives data only after A's output ends.
#[test]
fn sdk_output_drop_wakes_idle_input_and_isolates_calls() -> Result<(), ShellError> {
    for (byte_stream, gc_disabled) in [(false, false), (false, true), (true, false), (true, true)] {
        let sdk = SdkTest::new(InputPlugin {
            gc_disabled,
            background: None,
        })?;
        let peer = &sdk.peer;
        let output = &sdk.output;
        let mut output_ids = Vec::new();
        for call_id in 0..2 {
            sdk.run(call_id, "test-input", byte_stream)?;
            if gc_disabled {
                assert!(matches!(
                    output.0.recv_timeout(TIMEOUT),
                    Ok(PluginOutput::Option(PluginOption::GcDisabled(true)))
                ));
            }
            // Receiving a stream header proves run returned without cancelling its input.
            let response = output
                .0
                .recv_timeout(TIMEOUT)
                .expect("missing lazy response");
            let PluginOutput::CallResponse(id, PluginCallResponse::PipelineData(header)) = response
            else {
                panic!("unexpected response: {response:?}");
            };
            assert_eq!(id, call_id);
            let stream_id = match header {
                PipelineDataHeader::ListStream(info) if !byte_stream => info.id,
                PipelineDataHeader::ByteStream(info) if byte_stream => info.id,
                other => panic!("unexpected stream header: {other:?}"),
            };
            output_ids.push(stream_id);
        }

        peer.write(&PluginInput::Drop(output_ids[0]))?;
        let mut drops = [0; 2];
        loop {
            match output
                .0
                .recv_timeout(TIMEOUT)
                .expect("cancellation did not end A's output")
            {
                PluginOutput::Drop(10) => drops[0] += 1,
                PluginOutput::End(id) if id == output_ids[0] => break,
                other => panic!("unexpected output before B's data: {other:?}"),
            }
        }
        assert_eq!(drops[0], 1);
        // A's actual producer stayed connected and sent neither Data nor End until now.
        peer.write(&PluginInput::Drop(output_ids[0]))?;
        let data = input_data(byte_stream);
        peer.write(&PluginInput::Data(10, data.clone()))?;
        peer.write(&PluginInput::End(10))?;
        peer.write(&PluginInput::Data(11, data.clone()))?;
        peer.write(&PluginInput::End(11))?;
        let mut received_data = false;
        loop {
            match output.0.recv_timeout(TIMEOUT).expect("B did not complete") {
                PluginOutput::Ack(11) => (),
                PluginOutput::Drop(11) => drops[1] += 1,
                PluginOutput::Data(id, actual) if id == output_ids[1] => {
                    match (actual, &data) {
                        (StreamData::List(actual), StreamData::List(expected)) => {
                            assert_eq!(&actual, expected);
                        }
                        (StreamData::Raw(Ok(actual)), StreamData::Raw(Ok(expected))) => {
                            assert_eq!(&actual, expected);
                        }
                        other => panic!("unexpected stream data: {other:?}"),
                    }
                    received_data = true;
                    peer.write(&PluginInput::Ack(id))?;
                }
                PluginOutput::End(id) if id == output_ids[1] => break,
                other => panic!("unexpected output after late A messages: {other:?}"),
            }
        }
        assert!(received_data);
        assert_eq!(drops, [1, 1]);
        peer.write(&PluginInput::Drop(output_ids[1]))?;
        // Another protocol exchange succeeds after cancellation and both streams' End.
        peer.write(&PluginInput::Call(2, PluginCall::Metadata))?;
        assert!(matches!(
            output.0.recv_timeout(TIMEOUT),
            Ok(PluginOutput::CallResponse(
                2,
                PluginCallResponse::Metadata(_)
            ))
        ));
        sdk.stop()?;
    }
    Ok(())
}

/// Completion must release background readers for both successful and error responses, without
/// requiring Data, End, an interrupt, or transport closure. GC opt-out preserves those readers.
fn check_completion(name: &str, gc_disabled: bool) -> Result<(), ShellError> {
    for byte_stream in [false, true] {
        let fail = name == "background-error";
        let (background, background_rx) = mpsc::channel();
        let mut sdk = SdkTest::new(InputPlugin {
            gc_disabled,
            background: Some(background),
        })?;
        sdk.run(0, name, byte_stream)?;
        let reader = background_rx
            .recv_timeout(TIMEOUT)
            .expect("input thread was not started");
        reader
            .started
            .recv_timeout(TIMEOUT)
            .expect("input thread did not read");
        if gc_disabled {
            assert!(matches!(
                sdk.output.0.recv_timeout(TIMEOUT),
                Ok(PluginOutput::Option(PluginOption::GcDisabled(true)))
            ));
        }
        match sdk
            .output
            .0
            .recv_timeout(TIMEOUT)
            .expect("missing response")
        {
            PluginOutput::CallResponse(0, PluginCallResponse::Error(error)) if fail => {
                assert!(error.to_string().contains("background input failure"));
            }
            PluginOutput::CallResponse(
                0,
                PluginCallResponse::PipelineData(PipelineDataHeader::Empty),
            ) if name == "background" => (),
            PluginOutput::CallResponse(
                0,
                PluginCallResponse::PipelineData(PipelineDataHeader::ListStream(info)),
            ) if name == "background-stream" => {
                // A Drop also acknowledges normal End. It must not revoke GC opt-out.
                assert!(matches!(sdk.output.0.recv_timeout(TIMEOUT),
                            Ok(PluginOutput::End(id)) if id == info.id));
                sdk.peer.write(&PluginInput::Drop(info.id))?;
            }
            other => panic!("unexpected response: {other:?}"),
        }
        if gc_disabled {
            // Goodbye closes call intake, not the transport. Wait for all SDK runners to pass
            // finish_input before producing data, so scheduling cannot hide a premature cancel.
            sdk.finish_calls()?;
            assert!(!reader.finish(&sdk.peer, Some(input_data(byte_stream)))?);
        } else {
            assert!(matches!(
                reader.finish(&sdk.peer, None),
                Err(ShellError::Interrupted { .. })
            ));
        }
        let mut drops = 0;
        if !gc_disabled {
            sdk.peer
                .write(&PluginInput::Call(1, PluginCall::Metadata))?;
        }
        loop {
            let message = if gc_disabled {
                // Both SDK runners and the background reader have joined; their writes are done.
                match sdk.output.0.try_recv() {
                    Ok(message) => message,
                    Err(_) => break,
                }
            } else {
                sdk.output
                    .0
                    .recv_timeout(TIMEOUT)
                    .expect("protocol did not remain usable")
            };
            match message {
                PluginOutput::Ack(10) if gc_disabled => (),
                PluginOutput::Drop(10) => drops += 1,
                PluginOutput::CallResponse(1, PluginCallResponse::Metadata(_)) if !gc_disabled => {
                    break;
                }
                other => panic!("unexpected output after completion: {other:?}"),
            }
        }
        assert_eq!(drops, 1);
        sdk.stop()?;
    }
    Ok(())
}

#[test]
fn sdk_completion_cancels_background_input() -> Result<(), ShellError> {
    check_completion("background", false)
}

#[test]
fn sdk_completion_cancels_background_input_after_error() -> Result<(), ShellError> {
    check_completion("background-error", false)
}

#[test]
fn sdk_completion_cancels_background_input_after_stream() -> Result<(), ShellError> {
    check_completion("background-stream", false)
}

#[test]
fn sdk_completion_preserves_background_input_with_gc_disabled() -> Result<(), ShellError> {
    check_completion("background", true)
}

#[test]
fn sdk_completion_preserves_background_input_after_error_with_gc_disabled() -> Result<(), ShellError>
{
    check_completion("background-error", true)
}

#[test]
fn sdk_completion_preserves_background_input_after_stream_with_gc_disabled()
-> Result<(), ShellError> {
    check_completion("background-stream", true)
}
