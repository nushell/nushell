//! Interface used by the engine to communicate with the plugin.

use std::{
    collections::{btree_map, BTreeMap},
    sync::{mpsc, Arc},
};

use nu_protocol::{
    IntoInterruptiblePipelineData, ListStream, PipelineData, PluginSignature, ShellError, Spanned,
    Value,
};

use crate::{
    plugin::{context::PluginExecutionContext, PluginIdentity},
    protocol::{
        CallInfo, CustomValueOp, PluginCall, PluginCallId, PluginCallResponse, PluginCustomValue,
        PluginInput, PluginOutput, ProtocolInfo,
    },
    sequence::Sequence,
};

use super::{
    stream::{StreamManager, StreamManagerHandle},
    Interface, InterfaceManager, PipelineDataWriter, PluginRead, PluginWrite,
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
enum ReceivedPluginCallMessage {
    /// The final response to send
    Response(PluginCallResponse<PipelineData>),

    /// An critical error with the interface
    Error(ShellError),
}

/// Context for plugin call execution
#[derive(Clone)]
pub(crate) struct Context(Arc<dyn PluginExecutionContext>);

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Context")
    }
}

impl std::ops::Deref for Context {
    type Target = dyn PluginExecutionContext;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// Internal shared state between the manager and each interface.
struct PluginInterfaceState {
    /// The identity of the plugin being interfaced with
    identity: Arc<PluginIdentity>,
    /// Sequence for generating plugin call ids
    plugin_call_id_sequence: Sequence,
    /// Sequence for generating stream ids
    stream_id_sequence: Sequence,
    /// Sender to subscribe to a plugin call response
    plugin_call_subscription_sender: mpsc::Sender<(PluginCallId, PluginCallSubscription)>,
    /// The synchronized output writer
    writer: Box<dyn PluginWrite<PluginInput>>,
}

impl std::fmt::Debug for PluginInterfaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginInterfaceState")
            .field("identity", &self.identity)
            .field("plugin_call_id_sequence", &self.plugin_call_id_sequence)
            .field("stream_id_sequence", &self.stream_id_sequence)
            .field(
                "plugin_call_subscription_sender",
                &self.plugin_call_subscription_sender,
            )
            .finish_non_exhaustive()
    }
}

/// Sent to the [`PluginInterfaceManager`] before making a plugin call to indicate interest in its
/// response.
#[derive(Debug)]
struct PluginCallSubscription {
    /// The sender back to the thread that is waiting for the plugin call response
    sender: mpsc::Sender<ReceivedPluginCallMessage>,
    /// Optional context for the environment of a plugin call
    context: Option<Context>,
}

/// Manages reading and dispatching messages for [`PluginInterface`]s.
#[derive(Debug)]
pub(crate) struct PluginInterfaceManager {
    /// Shared state
    state: Arc<PluginInterfaceState>,
    /// Manages stream messages and state
    stream_manager: StreamManager,
    /// Protocol version info, set after `Hello` received
    protocol_info: Option<ProtocolInfo>,
    /// Subscriptions for messages related to plugin calls
    plugin_call_subscriptions: BTreeMap<PluginCallId, PluginCallSubscription>,
    /// Receiver for plugin call subscriptions
    plugin_call_subscription_receiver: mpsc::Receiver<(PluginCallId, PluginCallSubscription)>,
}

impl PluginInterfaceManager {
    pub(crate) fn new(
        identity: Arc<PluginIdentity>,
        writer: impl PluginWrite<PluginInput> + 'static,
    ) -> PluginInterfaceManager {
        let (subscription_tx, subscription_rx) = mpsc::channel();

        PluginInterfaceManager {
            state: Arc::new(PluginInterfaceState {
                identity,
                plugin_call_id_sequence: Sequence::default(),
                stream_id_sequence: Sequence::default(),
                plugin_call_subscription_sender: subscription_tx,
                writer: Box::new(writer),
            }),
            stream_manager: StreamManager::new(),
            protocol_info: None,
            plugin_call_subscriptions: BTreeMap::new(),
            plugin_call_subscription_receiver: subscription_rx,
        }
    }

    /// Consume pending messages in the `plugin_call_subscription_receiver`
    fn receive_plugin_call_subscriptions(&mut self) {
        while let Ok((id, subscription)) = self.plugin_call_subscription_receiver.try_recv() {
            if let btree_map::Entry::Vacant(e) = self.plugin_call_subscriptions.entry(id) {
                e.insert(subscription);
            } else {
                log::warn!("Duplicate plugin call ID ignored: {id}");
            }
        }
    }

    /// Find the context corresponding to the given plugin call id
    fn get_context(&mut self, id: PluginCallId) -> Result<Option<Context>, ShellError> {
        // Make sure we're up to date
        self.receive_plugin_call_subscriptions();
        // Find the subscription and return the context
        self.plugin_call_subscriptions
            .get(&id)
            .map(|sub| sub.context.clone())
            .ok_or_else(|| ShellError::PluginFailedToDecode {
                msg: format!("Unknown plugin call ID: {id}"),
            })
    }

    /// Send a [`PluginCallResponse`] to the appropriate sender
    fn send_plugin_call_response(
        &mut self,
        id: PluginCallId,
        response: PluginCallResponse<PipelineData>,
    ) -> Result<(), ShellError> {
        // Ensure we're caught up on the subscriptions made
        self.receive_plugin_call_subscriptions();

        // Remove the subscription, since this would be the last message
        if let Some(subscription) = self.plugin_call_subscriptions.remove(&id) {
            if subscription
                .sender
                .send(ReceivedPluginCallMessage::Response(response))
                .is_err()
            {
                log::warn!("Received a plugin call response for id={id}, but the caller hung up");
            }
            Ok(())
        } else {
            Err(ShellError::PluginFailedToDecode {
                msg: format!("Unknown plugin call ID: {id}"),
            })
        }
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
        mut reader: impl PluginRead<PluginOutput>,
    ) -> Result<(), ShellError> {
        while let Some(msg) = reader.read().transpose() {
            if self.is_finished() {
                break;
            }

            if let Err(err) = msg.and_then(|msg| self.consume(msg)) {
                // Error to streams
                let _ = self.stream_manager.broadcast_read_error(err.clone());
                // Error to call waiters
                self.receive_plugin_call_subscriptions();
                for subscription in
                    std::mem::take(&mut self.plugin_call_subscriptions).into_values()
                {
                    let _ = subscription
                        .sender
                        .send(ReceivedPluginCallMessage::Error(err.clone()));
                }
                return Err(err);
            }
        }
        Ok(())
    }
}

impl InterfaceManager for PluginInterfaceManager {
    type Interface = PluginInterface;
    type Input = PluginOutput;

    fn get_interface(&self) -> Self::Interface {
        PluginInterface {
            state: self.state.clone(),
            stream_manager_handle: self.stream_manager.get_handle(),
        }
    }

    fn consume(&mut self, input: Self::Input) -> Result<(), ShellError> {
        log::trace!("from plugin: {:?}", input);

        match input {
            PluginOutput::Hello(info) => {
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
                            info.version, local_info.version
                        ),
                    })
                }
            }
            _ if self.protocol_info.is_none() => {
                // Must send protocol info first
                Err(ShellError::PluginFailedToLoad {
                    msg: "Failed to receive initial Hello message. \
                        This plugin might be too old"
                        .into(),
                })
            }
            PluginOutput::Stream(message) => self.consume_stream_message(message),
            PluginOutput::CallResponse(id, response) => {
                // Handle reading the pipeline data, if any
                let response = match response {
                    PluginCallResponse::Error(err) => PluginCallResponse::Error(err),
                    PluginCallResponse::Signature(sigs) => PluginCallResponse::Signature(sigs),
                    PluginCallResponse::PipelineData(data) => {
                        // If there's an error with initializing this stream, change it to a plugin
                        // error response, but send it anyway
                        let exec_context = self.get_context(id)?;
                        let ctrlc = exec_context.as_ref().and_then(|c| c.0.ctrlc());
                        match self.read_pipeline_data(data, ctrlc) {
                            Ok(data) => PluginCallResponse::PipelineData(data),
                            Err(err) => PluginCallResponse::Error(err.into()),
                        }
                    }
                };
                self.send_plugin_call_response(id, response)
            }
        }
    }

    fn stream_manager(&self) -> &StreamManager {
        &self.stream_manager
    }

    fn prepare_pipeline_data(&self, mut data: PipelineData) -> Result<PipelineData, ShellError> {
        // Add source to any values
        match data {
            PipelineData::Value(ref mut value, _) => {
                PluginCustomValue::add_source(value, &self.state.identity);
                Ok(data)
            }
            PipelineData::ListStream(ListStream { stream, ctrlc, .. }, meta) => {
                let identity = self.state.identity.clone();
                Ok(stream
                    .map(move |mut value| {
                        PluginCustomValue::add_source(&mut value, &identity);
                        value
                    })
                    .into_pipeline_data_with_metadata(meta, ctrlc))
            }
            PipelineData::Empty | PipelineData::ExternalStream { .. } => Ok(data),
        }
    }
}

/// A reference through which a plugin can be interacted with during execution.
#[derive(Debug, Clone)]
pub(crate) struct PluginInterface {
    /// Shared state
    state: Arc<PluginInterfaceState>,
    /// Handle to stream manager
    stream_manager_handle: StreamManagerHandle,
}

impl PluginInterface {
    /// Write the protocol info. This should be done after initialization
    pub(crate) fn hello(&self) -> Result<(), ShellError> {
        self.write(PluginInput::Hello(ProtocolInfo::default()))?;
        self.flush()
    }

    /// Tell the plugin it should not expect any more plugin calls and should terminate after it has
    /// finished processing the ones it has already received.
    ///
    /// Note that this is automatically called when the last existing `PluginInterface` is dropped.
    /// You probably do not need to call this manually.
    pub(crate) fn goodbye(&self) -> Result<(), ShellError> {
        self.write(PluginInput::Goodbye)?;
        self.flush()
    }

    /// Write a plugin call message. Returns the writer for the stream, and the receiver for
    /// messages (e.g. response) related to the plugin call
    fn write_plugin_call(
        &self,
        call: PluginCall<PipelineData>,
        context: Option<Context>,
    ) -> Result<
        (
            PipelineDataWriter<Self>,
            mpsc::Receiver<ReceivedPluginCallMessage>,
        ),
        ShellError,
    > {
        let id = self.state.plugin_call_id_sequence.next()?;
        let (tx, rx) = mpsc::channel();

        // Convert the call into one with a header and handle the stream, if necessary
        let (call, writer) = match call {
            PluginCall::Signature => (PluginCall::Signature, Default::default()),
            PluginCall::CustomValueOp(value, op) => {
                (PluginCall::CustomValueOp(value, op), Default::default())
            }
            PluginCall::Run(CallInfo {
                name,
                call,
                input,
                config,
            }) => {
                let (header, writer) = self.init_write_pipeline_data(input)?;
                (
                    PluginCall::Run(CallInfo {
                        name,
                        call,
                        input: header,
                        config,
                    }),
                    writer,
                )
            }
        };

        // Register the subscription to the response, and the context
        self.state
            .plugin_call_subscription_sender
            .send((
                id,
                PluginCallSubscription {
                    sender: tx,
                    context,
                },
            ))
            .map_err(|_| ShellError::NushellFailed {
                msg: "PluginInterfaceManager hung up and is no longer accepting plugin calls"
                    .into(),
            })?;

        // Write request
        self.write(PluginInput::Call(id, call))?;
        self.flush()?;

        Ok((writer, rx))
    }

    /// Read the channel for plugin call messages and handle them until the response is received.
    fn receive_plugin_call_response(
        &self,
        rx: mpsc::Receiver<ReceivedPluginCallMessage>,
    ) -> Result<PluginCallResponse<PipelineData>, ShellError> {
        if let Ok(msg) = rx.recv() {
            // Handle message from receiver
            match msg {
                ReceivedPluginCallMessage::Response(resp) => Ok(resp),
                ReceivedPluginCallMessage::Error(err) => Err(err),
            }
        } else {
            // If we fail to get a response
            Err(ShellError::PluginFailedToDecode {
                msg: "Failed to receive response to plugin call".into(),
            })
        }
    }

    /// Perform a plugin call. Input and output streams are handled automatically.
    fn plugin_call(
        &self,
        call: PluginCall<PipelineData>,
        context: &Option<Context>,
    ) -> Result<PluginCallResponse<PipelineData>, ShellError> {
        let (writer, rx) = self.write_plugin_call(call, context.clone())?;

        // Finish writing stream in the background
        writer.write_background()?;

        self.receive_plugin_call_response(rx)
    }

    /// Get the command signatures from the plugin.
    pub(crate) fn get_signature(&self) -> Result<Vec<PluginSignature>, ShellError> {
        match self.plugin_call(PluginCall::Signature, &None)? {
            PluginCallResponse::Signature(sigs) => Ok(sigs),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response to plugin Signature call".into(),
            }),
        }
    }

    /// Run the plugin with the given call and execution context.
    pub(crate) fn run(
        &self,
        call: CallInfo<PipelineData>,
        context: Arc<impl PluginExecutionContext + 'static>,
    ) -> Result<PipelineData, ShellError> {
        let context = Some(Context(context));
        match self.plugin_call(PluginCall::Run(call), &context)? {
            PluginCallResponse::PipelineData(data) => Ok(data),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response to plugin Run call".into(),
            }),
        }
    }

    /// Collapse a custom value to its base value.
    pub(crate) fn custom_value_to_base_value(
        &self,
        value: Spanned<PluginCustomValue>,
    ) -> Result<Value, ShellError> {
        let span = value.span;
        let call = PluginCall::CustomValueOp(value, CustomValueOp::ToBaseValue);
        match self.plugin_call(call, &None)? {
            PluginCallResponse::PipelineData(out_data) => Ok(out_data.into_value(span)),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response to plugin CustomValueOp::ToBaseValue call"
                    .into(),
            }),
        }
    }
}

impl Interface for PluginInterface {
    type Output = PluginInput;

    fn write(&self, input: PluginInput) -> Result<(), ShellError> {
        log::trace!("to plugin: {:?}", input);
        self.state.writer.write(&input)
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

    fn prepare_pipeline_data(&self, data: PipelineData) -> Result<PipelineData, ShellError> {
        // Validate the destination of values in the pipeline data
        match data {
            PipelineData::Value(mut value, meta) => {
                PluginCustomValue::verify_source(&mut value, &self.state.identity)?;
                Ok(PipelineData::Value(value, meta))
            }
            PipelineData::ListStream(ListStream { stream, ctrlc, .. }, meta) => {
                let identity = self.state.identity.clone();
                Ok(stream
                    .map(move |mut value| {
                        match PluginCustomValue::verify_source(&mut value, &identity) {
                            Ok(()) => value,
                            // Put the error in the stream instead
                            Err(err) => Value::error(err, value.span()),
                        }
                    })
                    .into_pipeline_data_with_metadata(meta, ctrlc))
            }
            PipelineData::Empty | PipelineData::ExternalStream { .. } => Ok(data),
        }
    }
}

impl Drop for PluginInterface {
    fn drop(&mut self) {
        // Automatically send `Goodbye` if there are no more interfaces. In that case there would be
        // only two copies of the state, one of which we hold, and one of which the manager holds.
        //
        // Our copy is about to be dropped, so there would only be one left, the manager. The
        // manager will never send any plugin calls, so we should let the plugin know that.
        if Arc::strong_count(&self.state) < 3 {
            if let Err(err) = self.goodbye() {
                log::warn!("Error during plugin Goodbye: {err}");
            }
        }
    }
}
