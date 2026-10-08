use super::fake_plugin_channel;
use nu_plugin::{EngineInterface, EvaluatedCall, Plugin, PluginCommand};
use nu_plugin_core::PluginWrite;
use nu_plugin_protocol::{
    ByteStreamInfo, CallInfo, ListStreamInfo, PipelineDataHeader, PluginCall, PluginCallResponse,
    PluginInput, PluginOption, PluginOutput, ProtocolInfo, StreamData,
};
use nu_protocol::{
    ByteStreamType, LabeledError, ListStream, PipelineData, ShellError, Signals, Signature, Span,
    Value,
};
use std::{sync::mpsc, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(5);

struct InputPlugin {
    gc_disabled: bool,
    background: Option<mpsc::Sender<BackgroundReader>>,
}
struct Passthrough;
struct BackgroundInput(&'static str);

struct BackgroundReader {
    started: mpsc::Receiver<()>,
    result: mpsc::Receiver<Result<bool, ShellError>>,
    thread: std::thread::JoinHandle<()>,
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
                PipelineData::ListStream(stream, _) => Ok(stream.into_iter().next().is_none()),
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
            Ok(PipelineData::list_stream(
                ListStream::new(std::iter::empty(), Span::test_data(), Signals::empty()),
                None,
            ))
        } else {
            Ok(PipelineData::empty())
        }
    }
}

/// Drive the real SDK's serve loop with the same in-memory transport as PluginTest. Both inputs
/// stay open and idle until A's output is dropped; B receives data only after A's output ends.
#[test]
fn sdk_output_drop_wakes_idle_input_and_isolates_calls() -> Result<(), ShellError> {
    for (byte_stream, gc_disabled) in [(false, false), (false, true), (true, false), (true, true)] {
        let (input_read, peer) = fake_plugin_channel::<PluginInput>();
        let (output, output_write) = fake_plugin_channel::<PluginOutput>();
        let (done, done_rx) = mpsc::channel();
        let runner = std::thread::spawn(move || {
            let result = nu_plugin::serve_plugin_io(
                &InputPlugin {
                    gc_disabled,
                    background: None,
                },
                "input-cancellation",
                move || input_read,
                move || output_write,
            );
            let _ = done.send(result);
        });
        peer.write(&PluginInput::Hello(ProtocolInfo::default()))?;
        assert!(matches!(
            output.0.recv_timeout(TIMEOUT),
            Ok(PluginOutput::Hello(_))
        ));

        let input_header = |id| {
            if byte_stream {
                PipelineDataHeader::byte_stream(ByteStreamInfo::new(
                    id,
                    Span::test_data(),
                    ByteStreamType::Binary,
                ))
            } else {
                PipelineDataHeader::list_stream(ListStreamInfo::new(id, Span::test_data()))
            }
        };
        let mut output_ids = Vec::new();
        for call_id in 0..2 {
            peer.write(&PluginInput::Call(
                call_id,
                PluginCall::Run(CallInfo {
                    name: "test-input".into(),
                    call: EvaluatedCall::new(Span::test_data()),
                    input: input_header(10 + call_id),
                }),
            ))?;
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
        let data = if byte_stream {
            StreamData::Raw(Ok(vec![1, 2]))
        } else {
            Value::test_int(42).into()
        };
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
        peer.write(&PluginInput::Goodbye)?;
        drop(peer);
        done_rx
            .recv_timeout(TIMEOUT)
            .expect("SDK did not shut down")
            .expect("SDK failed");
        runner.join().expect("SDK runner panicked");
    }
    Ok(())
}

/// Completion must release background readers for both successful and error responses, without
/// requiring Data, End, an interrupt, or transport closure. GC opt-out preserves those readers.
#[test]
fn sdk_completion_cleans_background_input_and_respects_gc_opt_out() -> Result<(), ShellError> {
    for byte_stream in [false, true] {
        for gc_disabled in [false, true] {
            for name in ["background", "background-error", "background-stream"] {
                let fail = name == "background-error";
                let (input_read, peer) = fake_plugin_channel::<PluginInput>();
                let (output, output_write) = fake_plugin_channel::<PluginOutput>();
                let (background, background_rx) = mpsc::channel();
                let (done, done_rx) = mpsc::channel();
                let runner = std::thread::spawn(move || {
                    let result = nu_plugin::serve_plugin_io(
                        &InputPlugin {
                            gc_disabled,
                            background: Some(background),
                        },
                        "input-cleanup",
                        move || input_read,
                        move || output_write,
                    );
                    let _ = done.send(result);
                });
                peer.write(&PluginInput::Hello(ProtocolInfo::default()))?;
                assert!(matches!(
                    output.0.recv_timeout(TIMEOUT),
                    Ok(PluginOutput::Hello(_))
                ));
                let input = if byte_stream {
                    PipelineDataHeader::byte_stream(ByteStreamInfo::new(
                        10,
                        Span::test_data(),
                        ByteStreamType::Binary,
                    ))
                } else {
                    PipelineDataHeader::list_stream(ListStreamInfo::new(10, Span::test_data()))
                };
                peer.write(&PluginInput::Call(
                    0,
                    PluginCall::Run(CallInfo {
                        name: name.into(),
                        call: EvaluatedCall::new(Span::test_data()),
                        input,
                    }),
                ))?;
                let reader = background_rx
                    .recv_timeout(TIMEOUT)
                    .expect("input thread was not started");
                reader
                    .started
                    .recv_timeout(TIMEOUT)
                    .expect("input thread did not read");
                if gc_disabled {
                    assert!(matches!(
                        output.0.recv_timeout(TIMEOUT),
                        Ok(PluginOutput::Option(PluginOption::GcDisabled(true)))
                    ));
                }
                match output.0.recv_timeout(TIMEOUT).expect("missing response") {
                    PluginOutput::CallResponse(0, PluginCallResponse::Error(error)) if fail => {
                        assert!(error.to_string().contains("background input failure"));
                    }
                    PluginOutput::CallResponse(
                        0,
                        PluginCallResponse::PipelineData(PipelineDataHeader::Empty),
                    ) if !fail => (),
                    PluginOutput::CallResponse(
                        0,
                        PluginCallResponse::PipelineData(PipelineDataHeader::ListStream(info)),
                    ) if name == "background-stream" => {
                        // A Drop also acknowledges normal End. It must not revoke GC opt-out.
                        assert!(matches!(output.0.recv_timeout(TIMEOUT),
                            Ok(PluginOutput::End(id)) if id == info.id));
                        peer.write(&PluginInput::Drop(info.id))?;
                    }
                    other => panic!("unexpected response: {other:?}"),
                }
                if gc_disabled {
                    let pending = reader.result.recv_timeout(Duration::from_millis(50));
                    // Always unblock the producer for teardown, even if the assertion fails.
                    let data = if byte_stream {
                        StreamData::Raw(Ok(vec![1, 2]))
                    } else {
                        Value::test_int(42).into()
                    };
                    peer.write(&PluginInput::Data(10, data))?;
                    peer.write(&PluginInput::End(10))?;
                    let result = reader.result.recv_timeout(TIMEOUT);
                    reader.thread.join().expect("input reader panicked");
                    assert!(matches!(pending, Err(mpsc::RecvTimeoutError::Timeout)));
                    assert!(!result.expect("GC opt-out did not preserve input")?);
                } else {
                    let result = reader.result.recv_timeout(TIMEOUT);
                    // The producer was still connected and idle when the result was obtained.
                    peer.write(&PluginInput::End(10))?;
                    reader.thread.join().expect("input reader panicked");
                    assert!(result.expect("completion did not release the input thread")?);
                }
                let mut drops = 0;
                peer.write(&PluginInput::Call(1, PluginCall::Metadata))?;
                loop {
                    match output
                        .0
                        .recv_timeout(TIMEOUT)
                        .expect("protocol did not remain usable")
                    {
                        PluginOutput::Ack(10) if gc_disabled => (),
                        PluginOutput::Drop(10) => drops += 1,
                        PluginOutput::CallResponse(1, PluginCallResponse::Metadata(_)) => break,
                        other => panic!("unexpected output after completion: {other:?}"),
                    }
                }
                assert_eq!(drops, 1);
                peer.write(&PluginInput::Goodbye)?;
                drop(peer);
                done_rx
                    .recv_timeout(TIMEOUT)
                    .expect("SDK did not shut down")
                    .expect("SDK failed");
                runner.join().expect("SDK runner panicked");
            }
        }
    }
    Ok(())
}
