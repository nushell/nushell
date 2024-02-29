//! Interface used by the plugin to communicate with the engine.

use std::sync::{mpsc, Arc};

use nu_protocol::{
    IntoInterruptiblePipelineData, ListStream, PipelineData, PluginSignature, ShellError, Spanned,
    Value,
};

use crate::{
    protocol::{
        CallInfo, CustomValueOp, PluginCall, PluginCallId, PluginCallResponse, PluginCustomValue,
        PluginInput, ProtocolInfo,
    },
    LabeledError, PluginOutput,
};

use super::{
    stream::{StreamManager, StreamManagerHandle},
    Interface, InterfaceManager, PipelineDataWriter, PluginRead, PluginWrite,
};
use crate::sequence::Sequence;

/// Plugin calls that are received by the [`EngineInterfaceManager`] for handling.
///
/// With each call, an [`EngineInterface`] is included that can be provided to the plugin code
/// and should be used to send the response. The interface sent includes the [`PluginCallId`] for
/// sending associated messages with the correct context.
#[derive(Debug)]
pub(crate) enum ReceivedPluginCall {
    Signature {
        engine: EngineInterface,
    },
    Run {
        engine: EngineInterface,
        call: CallInfo<PipelineData>,
    },
    CustomValueOp {
        engine: EngineInterface,
        custom_value: Spanned<PluginCustomValue>,
        op: CustomValueOp,
    },
}

#[cfg(test)]
mod tests;

/// Internal shared state between the manager and each interface.
struct EngineInterfaceState {
    /// Sequence for generating stream ids
    stream_id_sequence: Sequence,
    /// The synchronized output writer
    writer: Box<dyn PluginWrite<PluginOutput>>,
}

impl std::fmt::Debug for EngineInterfaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineInterfaceState")
            .field("stream_id_sequence", &self.stream_id_sequence)
            .finish_non_exhaustive()
    }
}

/// Manages reading and dispatching messages for [`EngineInterface`]s.
#[derive(Debug)]
pub(crate) struct EngineInterfaceManager {
    /// Shared state
    state: Arc<EngineInterfaceState>,
    /// Channel to send received PluginCalls to. This is removed after `Goodbye` is received.
    plugin_call_sender: Option<mpsc::Sender<ReceivedPluginCall>>,
    /// Receiver for PluginCalls. This is usually taken after initialization
    plugin_call_receiver: Option<mpsc::Receiver<ReceivedPluginCall>>,
    /// Manages stream messages and state
    stream_manager: StreamManager,
    /// Protocol version info, set after `Hello` received
    protocol_info: Option<ProtocolInfo>,
}

impl EngineInterfaceManager {
    pub(crate) fn new(writer: impl PluginWrite<PluginOutput> + 'static) -> EngineInterfaceManager {
        let (plug_tx, plug_rx) = mpsc::channel();

        EngineInterfaceManager {
            state: Arc::new(EngineInterfaceState {
                stream_id_sequence: Sequence::default(),
                writer: Box::new(writer),
            }),
            plugin_call_sender: Some(plug_tx),
            plugin_call_receiver: Some(plug_rx),
            stream_manager: StreamManager::new(),
            protocol_info: None,
        }
    }

    /// Get the receiving end of the plugin call channel. Plugin calls that need to be handled
    /// will be sent here.
    pub(crate) fn take_plugin_call_receiver(
        &mut self,
    ) -> Option<mpsc::Receiver<ReceivedPluginCall>> {
        self.plugin_call_receiver.take()
    }

    /// Create an [`EngineInterface`] associated with the given call id.
    fn interface_for_context(&self, context: PluginCallId) -> EngineInterface {
        EngineInterface {
            state: self.state.clone(),
            stream_manager_handle: self.stream_manager.get_handle(),
            context: Some(context),
        }
    }

    /// Send a [`ReceivedPluginCall`] to the channel
    fn send_plugin_call(&self, plugin_call: ReceivedPluginCall) -> Result<(), ShellError> {
        self.plugin_call_sender
            .as_ref()
            .ok_or_else(|| ShellError::PluginFailedToDecode {
                msg: "Received a plugin call after Goodbye".into(),
            })?
            .send(plugin_call)
            .map_err(|_| ShellError::NushellFailed {
                msg: "Received a plugin call, but there's nowhere to send it".into(),
            })
    }

    /// True if there are no other copies of the state (which would mean there are no interfaces
    /// and no stream readers/writers)
    pub(crate) fn is_finished(&self) -> bool {
        Arc::strong_count(&self.state) < 2
    }

    /// Loop on input from the given reader as long as `is_finished()` is false
    ///
    /// Any errors will be propagated to all read streams automatically.
    pub(crate) fn consume_all(
        &mut self,
        mut reader: impl PluginRead<PluginInput>,
    ) -> Result<(), ShellError> {
        while let Some(msg) = reader.read().transpose() {
            if self.is_finished() {
                break;
            }

            if let Err(err) = msg.and_then(|msg| self.consume(msg)) {
                let _ = self.stream_manager.broadcast_read_error(err.clone());
                return Err(err);
            }
        }
        Ok(())
    }
}

impl InterfaceManager for EngineInterfaceManager {
    type Interface = EngineInterface;
    type Input = PluginInput;

    fn get_interface(&self) -> Self::Interface {
        EngineInterface {
            state: self.state.clone(),
            stream_manager_handle: self.stream_manager.get_handle(),
            context: None,
        }
    }

    fn consume(&mut self, input: Self::Input) -> Result<(), ShellError> {
        log::trace!("from engine: {:?}", input);

        match input {
            PluginInput::Hello(info) => {
                let local_info = ProtocolInfo::default();
                if local_info.is_compatible_with(&info)? {
                    self.protocol_info = Some(info);
                    Ok(())
                } else {
                    self.protocol_info = None;
                    Err(ShellError::PluginFailedToLoad {
                        msg: format!(
                            "Plugin is compiled for nushell version {}, \
                                which is not compatible with version {}",
                            local_info.version, info.version
                        ),
                    })
                }
            }
            _ if self.protocol_info.is_none() => {
                // Must send protocol info first
                Err(ShellError::PluginFailedToLoad {
                    msg: "Failed to receive initial Hello message. This engine might be too old"
                        .into(),
                })
            }
            PluginInput::Stream(message) => self.consume_stream_message(message),
            PluginInput::Call(id, call) => match call {
                // We just let the receiver handle it rather than trying to store signature here
                // or something
                PluginCall::Signature => self.send_plugin_call(ReceivedPluginCall::Signature {
                    engine: self.interface_for_context(id),
                }),
                // Set up the streams from the input and reformat to a ReceivedPluginCall
                PluginCall::Run(CallInfo {
                    name,
                    mut call,
                    input,
                    config,
                }) => {
                    let interface = self.interface_for_context(id);
                    // If there's an error with initialization of the input stream, just send
                    // the error response rather than failing here
                    match self.read_pipeline_data(input, None) {
                        Ok(input) => {
                            // Deserialize custom values in the arguments
                            if let Err(err) = deserialize_call_args(&mut call) {
                                return interface.write_response(Err(err))?.write();
                            }
                            // Send the plugin call to the receiver
                            self.send_plugin_call(ReceivedPluginCall::Run {
                                engine: interface,
                                call: CallInfo {
                                    name,
                                    call,
                                    input,
                                    config,
                                },
                            })
                        }
                        err @ Err(_) => interface.write_response(err)?.write(),
                    }
                }
                // Send request with the custom value
                PluginCall::CustomValueOp(custom_value, op) => {
                    self.send_plugin_call(ReceivedPluginCall::CustomValueOp {
                        engine: self.interface_for_context(id),
                        custom_value,
                        op,
                    })
                }
            },
            PluginInput::Goodbye => {
                // Remove the plugin call sender so it hangs up
                drop(self.plugin_call_sender.take());
                Ok(())
            }
        }
    }

    fn stream_manager(&self) -> &StreamManager {
        &self.stream_manager
    }

    fn prepare_pipeline_data(&self, mut data: PipelineData) -> Result<PipelineData, ShellError> {
        // Deserialize custom values in the pipeline data
        match data {
            PipelineData::Value(ref mut value, _) => {
                PluginCustomValue::deserialize_custom_values_in(value)?;
                Ok(data)
            }
            PipelineData::ListStream(ListStream { stream, ctrlc, .. }, meta) => Ok(stream
                .map(|mut value| {
                    let span = value.span();
                    PluginCustomValue::deserialize_custom_values_in(&mut value)
                        .map(|()| value)
                        .unwrap_or_else(|err| Value::error(err, span))
                })
                .into_pipeline_data_with_metadata(meta, ctrlc)),
            PipelineData::Empty | PipelineData::ExternalStream { .. } => Ok(data),
        }
    }
}

/// Deserialize custom values in call arguments
fn deserialize_call_args(call: &mut crate::EvaluatedCall) -> Result<(), ShellError> {
    call.positional
        .iter_mut()
        .try_for_each(PluginCustomValue::deserialize_custom_values_in)?;
    call.named
        .iter_mut()
        .flat_map(|(_, value)| value.as_mut())
        .try_for_each(PluginCustomValue::deserialize_custom_values_in)
}

/// A reference through which the nushell engine can be interacted with during execution.
#[derive(Debug, Clone)]
pub struct EngineInterface {
    /// Shared state with the manager
    state: Arc<EngineInterfaceState>,
    /// Handle to stream manager
    stream_manager_handle: StreamManagerHandle,
    /// The plugin call this interface belongs to.
    context: Option<PluginCallId>,
}

impl EngineInterface {
    /// Write the protocol info. This should be done after initialization
    pub(crate) fn hello(&self) -> Result<(), ShellError> {
        self.write(PluginOutput::Hello(ProtocolInfo::default()))?;
        self.flush()
    }

    fn context(&self) -> Result<PluginCallId, ShellError> {
        self.context.ok_or_else(|| ShellError::NushellFailed {
            msg: "Tried to call an EngineInterface method that requires a call context \
                outside of one"
                .into(),
        })
    }

    /// Write a call response of either [`PipelineData`] or an error. Returns the stream writer
    /// to finish writing the stream
    pub(crate) fn write_response(
        &self,
        result: Result<PipelineData, impl Into<LabeledError>>,
    ) -> Result<PipelineDataWriter<Self>, ShellError> {
        match result {
            Ok(data) => {
                let (header, writer) = match self.init_write_pipeline_data(data) {
                    Ok(tup) => tup,
                    // If we get an error while trying to construct the pipeline data, send that
                    // instead
                    Err(err) => return self.write_response(Err(err)),
                };
                // Write pipeline data header response, and the full stream
                let response = PluginCallResponse::PipelineData(header);
                self.write(PluginOutput::CallResponse(self.context()?, response))?;
                self.flush()?;
                Ok(writer)
            }
            Err(err) => {
                let response = PluginCallResponse::Error(err.into());
                self.write(PluginOutput::CallResponse(self.context()?, response))?;
                self.flush()?;
                Ok(Default::default())
            }
        }
    }

    /// Write a call response of plugin signatures.
    pub(crate) fn write_signature(
        &self,
        signature: Vec<PluginSignature>,
    ) -> Result<(), ShellError> {
        let response = PluginCallResponse::Signature(signature);
        self.write(PluginOutput::CallResponse(self.context()?, response))?;
        self.flush()
    }
}

impl Interface for EngineInterface {
    type Output = PluginOutput;

    fn write(&self, output: PluginOutput) -> Result<(), ShellError> {
        log::trace!("to engine: {:?}", output);
        self.state.writer.write(&output)
    }

    fn flush(&self) -> Result<(), ShellError> {
        self.state.writer.flush()
    }

    fn stream_id_sequence(&self) -> &Sequence {
        &self.state.stream_id_sequence
    }

    fn stream_manager_handle(&self) -> &StreamManagerHandle {
        &self.stream_manager_handle
    }

    fn prepare_pipeline_data(&self, mut data: PipelineData) -> Result<PipelineData, ShellError> {
        // Serialize custom values in the pipeline data
        match data {
            PipelineData::Value(ref mut value, _) => {
                PluginCustomValue::serialize_custom_values_in(value)?;
                Ok(data)
            }
            PipelineData::ListStream(ListStream { stream, ctrlc, .. }, meta) => Ok(stream
                .map(|mut value| {
                    let span = value.span();
                    PluginCustomValue::serialize_custom_values_in(&mut value)
                        .map(|_| value)
                        .unwrap_or_else(|err| Value::error(err, span))
                })
                .into_pipeline_data_with_metadata(meta, ctrlc)),
            PipelineData::Empty | PipelineData::ExternalStream { .. } => Ok(data),
        }
    }
}
