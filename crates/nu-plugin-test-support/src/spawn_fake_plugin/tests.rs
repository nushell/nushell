use super::fake_plugin_channel;
use nu_plugin::{EngineInterface, EvaluatedCall, InputCancellation, Plugin, PluginCommand};
use nu_plugin_core::PluginWrite;
use nu_plugin_protocol::{
    ByteStreamInfo, CallInfo, ListStreamInfo, PipelineDataHeader, PluginCall, PluginCallResponse,
    PluginInput, PluginOutput, ProtocolInfo, StreamData,
};
use nu_protocol::{ByteStreamType, LabeledError, PipelineData, ShellError, Signature, Span, Value};
use std::{sync::mpsc, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(5);

struct InputPlugin(mpsc::Sender<InputCancellation>);
struct Passthrough;

impl Plugin for InputPlugin {
    fn version(&self) -> String {
        "0.0.0".into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Passthrough)]
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
        let cancellation = engine
            .input_cancellation()
            .ok_or_else(|| LabeledError::new("Expected a transport input"))?;
        plugin
            .0
            .send(cancellation)
            .map_err(|err| LabeledError::new(err.to_string()))?;
        Ok(input)
    }
}

/// Drive the real SDK's serve loop with the same in-memory transport as PluginTest. Both inputs
/// stay open and idle until A is cancelled; B receives its data only after A's output ends.
#[test]
fn sdk_input_cancellation_after_lazy_response_is_isolated() -> Result<(), ShellError> {
    for byte_stream in [false, true] {
        let (input_read, peer) = fake_plugin_channel::<PluginInput>();
        let (output, output_write) = fake_plugin_channel::<PluginOutput>();
        let (handles, handles_rx) = mpsc::channel();
        let (done, done_rx) = mpsc::channel();
        let runner = std::thread::spawn(move || {
            let result = nu_plugin::serve_plugin_io(
                &InputPlugin(handles),
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
        let mut cancellations = Vec::new();
        for call_id in 0..2 {
            peer.write(&PluginInput::Call(
                call_id,
                PluginCall::Run(CallInfo {
                    name: "test-input".into(),
                    call: EvaluatedCall::new(Span::test_data()),
                    input: input_header(10 + call_id),
                }),
            ))?;
            cancellations.push(
                handles_rx
                    .recv_timeout(TIMEOUT)
                    .expect("command did not receive handle"),
            );
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

        cancellations[0].cancel();
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
        cancellations[0].cancel();
        cancellations[1].cancel();
    }
    Ok(())
}
