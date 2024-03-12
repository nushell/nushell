//! Interface used by the engine to communicate with the plugin.

use std::{
    collections::{btree_map, BTreeMap},
    sync::{mpsc, Arc, OnceLock},
};

use nu_protocol::{
    ast::Operator, IntoInterruptiblePipelineData, IntoSpanned, ListStream, PipelineData,
    PluginSignature, ShellError, Span, Spanned, Value,
};

use crate::{
    plugin::{context::PluginExecutionContext, gc::PluginGc, PluginSource},
    protocol::{
        CallInfo, CustomValueOp, EngineCall, EngineCallId, EngineCallResponse, Ordering,
        PluginCall, PluginCallId, PluginCallResponse, PluginCustomValue, PluginInput, PluginOption,
        PluginOutput, ProtocolInfo, StreamId, StreamMessage,
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

    /// An engine call that should be evaluated and responded to, but is not the final response
    ///
    /// We send this back to the thread that made the plugin call so we don't block the reader
    /// thread
    EngineCall(EngineCallId, EngineCall<PipelineData>),
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
    /// The source to be used for custom values coming from / going to the plugin
    source: Arc<PluginSource>,
    /// Sequence for generating plugin call ids
    plugin_call_id_sequence: Sequence,
    /// Sequence for generating stream ids
    stream_id_sequence: Sequence,
    /// Sender to subscribe to a plugin call response
    plugin_call_subscription_sender: mpsc::Sender<(PluginCallId, PluginCallSubscription)>,
    /// An error that should be propagated to further plugin calls
    error: OnceLock<ShellError>,
    /// The synchronized output writer
    writer: Box<dyn PluginWrite<PluginInput>>,
}

impl std::fmt::Debug for PluginInterfaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginInterfaceState")
            .field("source", &self.source)
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
    sender: Option<mpsc::Sender<ReceivedPluginCallMessage>>,
    /// Optional context for the environment of a plugin call for servicing engine calls
    context: Option<Context>,
    /// Number of streams that still need to be read from the plugin call response
    remaining_streams_to_read: i32,
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
    /// Tracker for which plugin call streams being read belong to
    ///
    /// This is necessary so we know when we can remove context for plugin calls
    plugin_call_input_streams: BTreeMap<StreamId, PluginCallId>,
    /// Garbage collector handle, to notify about the state of the plugin
    gc: Option<PluginGc>,
}

impl PluginInterfaceManager {
    pub(crate) fn new(
        source: Arc<PluginSource>,
        writer: impl PluginWrite<PluginInput> + 'static,
    ) -> PluginInterfaceManager {
        let (subscription_tx, subscription_rx) = mpsc::channel();

        PluginInterfaceManager {
            state: Arc::new(PluginInterfaceState {
                source,
                plugin_call_id_sequence: Sequence::default(),
                stream_id_sequence: Sequence::default(),
                plugin_call_subscription_sender: subscription_tx,
                error: OnceLock::new(),
                writer: Box::new(writer),
            }),
            stream_manager: StreamManager::new(),
            protocol_info: None,
            plugin_call_subscriptions: BTreeMap::new(),
            plugin_call_subscription_receiver: subscription_rx,
            plugin_call_input_streams: BTreeMap::new(),
            gc: None,
        }
    }

    /// Add a garbage collector to this plugin. The manager will notify the garbage collector about
    /// the state of the plugin so that it can be automatically cleaned up if the plugin is
    /// inactive.
    pub(crate) fn set_garbage_collector(&mut self, gc: Option<PluginGc>) {
        self.gc = gc;
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

    /// Track the start of incoming stream(s)
    fn recv_stream_started(&mut self, call_id: PluginCallId, stream_id: StreamId) {
        self.plugin_call_input_streams.insert(stream_id, call_id);
        // Increment the number of streams on the subscription so context stays alive
        self.receive_plugin_call_subscriptions();
        if let Some(sub) = self.plugin_call_subscriptions.get_mut(&call_id) {
            sub.remaining_streams_to_read += 1;
        }
        // Add a lock to the garbage collector for each stream
        if let Some(ref gc) = self.gc {
            gc.increment_locks(1);
        }
    }

    /// Track the end of an incoming stream
    fn recv_stream_ended(&mut self, stream_id: StreamId) {
        if let Some(call_id) = self.plugin_call_input_streams.remove(&stream_id) {
            if let btree_map::Entry::Occupied(mut e) = self.plugin_call_subscriptions.entry(call_id)
            {
                e.get_mut().remaining_streams_to_read -= 1;
                // Remove the subscription if there are no more streams to be read.
                if e.get().remaining_streams_to_read <= 0 {
                    e.remove();
                }
            }
            // Streams read from the plugin are tracked with locks on the GC so plugins don't get
            // stopped if they have active streams
            if let Some(ref gc) = self.gc {
                gc.decrement_locks(1);
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

        if let btree_map::Entry::Occupied(mut e) = self.plugin_call_subscriptions.entry(id) {
            // Remove the subscription sender, since this will be the last message.
            //
            // We can spawn a new one if we need it for engine calls.
            if e.get_mut()
                .sender
                .take()
                .and_then(|s| s.send(ReceivedPluginCallMessage::Response(response)).ok())
                .is_none()
            {
                log::warn!("Received a plugin call response for id={id}, but the caller hung up");
            }
            // If there are no registered streams, just remove it
            if e.get().remaining_streams_to_read <= 0 {
                e.remove();
            }
            Ok(())
        } else {
            Err(ShellError::PluginFailedToDecode {
                msg: format!("Unknown plugin call ID: {id}"),
            })
        }
    }

    /// Spawn a handler for engine calls for a plugin, in case we need to handle engine calls
    /// after the response has already been received (in which case we have nowhere to send them)
    fn spawn_engine_call_handler(
        &mut self,
        id: PluginCallId,
    ) -> Result<&mpsc::Sender<ReceivedPluginCallMessage>, ShellError> {
        let interface = self.get_interface();

        if let Some(sub) = self.plugin_call_subscriptions.get_mut(&id) {
            if sub.sender.is_none() {
                let (tx, rx) = mpsc::channel();
                let context = sub.context.clone();
                let handler = move || {
                    for msg in rx {
                        // This thread only handles engine calls.
                        match msg {
                            ReceivedPluginCallMessage::EngineCall(engine_call_id, engine_call) => {
                                if let Err(err) = interface.handle_engine_call(
                                    engine_call_id,
                                    engine_call,
                                    &context,
                                ) {
                                    log::warn!(
                                        "Error in plugin post-response engine call handler: \
                                        {err:?}"
                                    );
                                    return;
                                }
                            }
                            other => log::warn!(
                                "Bad message received in plugin post-response \
                                engine call handler: {other:?}"
                            ),
                        }
                    }
                };
                std::thread::Builder::new()
                    .name("plugin engine call handler".into())
                    .spawn(handler)
                    .expect("failed to spawn thread");
                sub.sender = Some(tx);
                Ok(sub.sender.as_ref().unwrap_or_else(|| unreachable!()))
            } else {
                Err(ShellError::NushellFailed {
                    msg: "Tried to spawn the fallback engine call handler before the plugin call \
                        response had been received"
                        .into(),
                })
            }
        } else {
            Err(ShellError::NushellFailed {
                msg: format!("Couldn't find plugin ID={id} in subscriptions"),
            })
        }
    }

    /// Send an [`EngineCall`] to the appropriate sender
    fn send_engine_call(
        &mut self,
        plugin_call_id: PluginCallId,
        engine_call_id: EngineCallId,
        call: EngineCall<PipelineData>,
    ) -> Result<(), ShellError> {
        // Ensure we're caught up on the subscriptions made
        self.receive_plugin_call_subscriptions();

        // Don't remove the sender, as there could be more calls or responses
        if let Some(subscription) = self.plugin_call_subscriptions.get(&plugin_call_id) {
            let msg = ReceivedPluginCallMessage::EngineCall(engine_call_id, call);
            // Call if there's an error sending the engine call
            let send_error = |this: &Self| {
                log::warn!(
                    "Received an engine call for plugin_call_id={plugin_call_id}, \
                    but the caller hung up"
                );
                // We really have no choice here but to send the response ourselves and hope we
                // don't block
                this.state.writer.write(&PluginInput::EngineCallResponse(
                    engine_call_id,
                    EngineCallResponse::Error(ShellError::IOError {
                        msg: "Can't make engine call because the original caller hung up".into(),
                    }),
                ))?;
                this.state.writer.flush()
            };
            // Try to send to the sender if it exists
            if let Some(sender) = subscription.sender.as_ref() {
                sender.send(msg).or_else(|_| send_error(self))
            } else {
                // The sender no longer exists. Spawn a specific one just for engine calls
                let sender = self.spawn_engine_call_handler(plugin_call_id)?;
                sender.send(msg).or_else(|_| send_error(self))
            }
        } else {
            Err(ShellError::PluginFailedToDecode {
                msg: format!("Unknown plugin call ID: {plugin_call_id}"),
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
        let mut result = Ok(());

        while let Some(msg) = reader.read().transpose() {
            if self.is_finished() {
                break;
            }

            // We assume an error here is unrecoverable (at least, without restarting the plugin)
            if let Err(err) = msg.and_then(|msg| self.consume(msg)) {
                // Put the error in the state so that new calls see it
                let _ = self.state.error.set(err.clone());
                // Error to streams
                let _ = self.stream_manager.broadcast_read_error(err.clone());
                // Error to call waiters
                self.receive_plugin_call_subscriptions();
                for subscription in
                    std::mem::take(&mut self.plugin_call_subscriptions).into_values()
                {
                    let _ = subscription
                        .sender
                        .as_ref()
                        .map(|s| s.send(ReceivedPluginCallMessage::Error(err.clone())));
                }
                result = Err(err);
                break;
            }
        }

        // Tell the GC we are exiting so that the plugin doesn't get stuck open
        if let Some(ref gc) = self.gc {
            gc.exited();
        }
        result
    }
}

impl InterfaceManager for PluginInterfaceManager {
    type Interface = PluginInterface;
    type Input = PluginOutput;

    fn get_interface(&self) -> Self::Interface {
        PluginInterface {
            state: self.state.clone(),
            stream_manager_handle: self.stream_manager.get_handle(),
            gc: self.gc.clone(),
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
                            "Plugin `{}` is compiled for nushell version {}, \
                                which is not compatible with version {}",
                            self.state.source.name(),
                            info.version,
                            local_info.version,
                        ),
                    })
                }
            }
            _ if self.protocol_info.is_none() => {
                // Must send protocol info first
                Err(ShellError::PluginFailedToLoad {
                    msg: format!(
                        "Failed to receive initial Hello message from `{}`. \
                            This plugin might be too old",
                        self.state.source.name()
                    ),
                })
            }
            PluginOutput::Stream(message) => self.consume_stream_message(message),
            PluginOutput::Option(option) => match option {
                PluginOption::GcDisabled(disabled) => {
                    // Turn garbage collection off/on.
                    if let Some(ref gc) = self.gc {
                        gc.set_disabled(disabled);
                    }
                    Ok(())
                }
            },
            PluginOutput::CallResponse(id, response) => {
                // Handle reading the pipeline data, if any
                let response = match response {
                    PluginCallResponse::Error(err) => PluginCallResponse::Error(err),
                    PluginCallResponse::Signature(sigs) => PluginCallResponse::Signature(sigs),
                    PluginCallResponse::Ordering(ordering) => {
                        PluginCallResponse::Ordering(ordering)
                    }
                    PluginCallResponse::PipelineData(data) => {
                        // If there's an error with initializing this stream, change it to a plugin
                        // error response, but send it anyway
                        let exec_context = self.get_context(id)?;
                        let ctrlc = exec_context.as_ref().and_then(|c| c.0.ctrlc());

                        // Register the streams in the response
                        for stream_id in data.stream_ids() {
                            self.recv_stream_started(id, stream_id);
                        }

                        match self.read_pipeline_data(data, ctrlc) {
                            Ok(data) => PluginCallResponse::PipelineData(data),
                            Err(err) => PluginCallResponse::Error(err.into()),
                        }
                    }
                };
                let result = self.send_plugin_call_response(id, response);
                if result.is_ok() {
                    // When a call ends, it releases a lock on the GC
                    if let Some(ref gc) = self.gc {
                        gc.decrement_locks(1);
                    }
                }
                result
            }
            PluginOutput::EngineCall { context, id, call } => {
                // Handle reading the pipeline data, if any
                let exec_context = self.get_context(context)?;
                let ctrlc = exec_context.as_ref().and_then(|c| c.0.ctrlc());
                let call = match call {
                    EngineCall::GetConfig => Ok(EngineCall::GetConfig),
                    EngineCall::GetPluginConfig => Ok(EngineCall::GetPluginConfig),
                    EngineCall::GetEnvVar(name) => Ok(EngineCall::GetEnvVar(name)),
                    EngineCall::GetEnvVars => Ok(EngineCall::GetEnvVars),
                    EngineCall::GetCurrentDir => Ok(EngineCall::GetCurrentDir),
                    EngineCall::EvalClosure {
                        closure,
                        mut positional,
                        input,
                        redirect_stdout,
                        redirect_stderr,
                    } => {
                        // Add source to any plugin custom values in the arguments
                        for arg in positional.iter_mut() {
                            PluginCustomValue::add_source(arg, &self.state.source);
                        }
                        self.read_pipeline_data(input, ctrlc)
                            .map(|input| EngineCall::EvalClosure {
                                closure,
                                positional,
                                input,
                                redirect_stdout,
                                redirect_stderr,
                            })
                    }
                };
                match call {
                    Ok(call) => self.send_engine_call(context, id, call),
                    // If there was an error with setting up the call, just write the error
                    Err(err) => self
                        .get_interface()
                        .write_engine_call_response(id, EngineCallResponse::Error(err)),
                }
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
                PluginCustomValue::add_source(value, &self.state.source);
                Ok(data)
            }
            PipelineData::ListStream(ListStream { stream, ctrlc, .. }, meta) => {
                let source = self.state.source.clone();
                Ok(stream
                    .map(move |mut value| {
                        PluginCustomValue::add_source(&mut value, &source);
                        value
                    })
                    .into_pipeline_data_with_metadata(meta, ctrlc))
            }
            PipelineData::Empty | PipelineData::ExternalStream { .. } => Ok(data),
        }
    }

    fn consume_stream_message(&mut self, message: StreamMessage) -> Result<(), ShellError> {
        // Keep track of streams that end
        if let StreamMessage::End(id) = message {
            self.recv_stream_ended(id);
        }
        self.stream_manager.handle_message(message)
    }
}

/// A reference through which a plugin can be interacted with during execution.
#[derive(Debug, Clone)]
pub(crate) struct PluginInterface {
    /// Shared state
    state: Arc<PluginInterfaceState>,
    /// Handle to stream manager
    stream_manager_handle: StreamManagerHandle,
    /// Handle to plugin garbage collector
    gc: Option<PluginGc>,
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

    /// Write an [`EngineCallResponse`]. Writes the full stream contained in any [`PipelineData`]
    /// before returning.
    pub(crate) fn write_engine_call_response(
        &self,
        id: EngineCallId,
        response: EngineCallResponse<PipelineData>,
    ) -> Result<(), ShellError> {
        // Set up any stream if necessary
        let (response, writer) = match response {
            EngineCallResponse::PipelineData(data) => {
                let (header, writer) = self.init_write_pipeline_data(data)?;
                (EngineCallResponse::PipelineData(header), Some(writer))
            }
            // No pipeline data:
            EngineCallResponse::Error(err) => (EngineCallResponse::Error(err), None),
            EngineCallResponse::Config(config) => (EngineCallResponse::Config(config), None),
            EngineCallResponse::ValueMap(map) => (EngineCallResponse::ValueMap(map), None),
        };

        // Write the response, including the pipeline data header if present
        self.write(PluginInput::EngineCallResponse(id, response))?;
        self.flush()?;

        // If we have a stream to write, do it now
        if let Some(writer) = writer {
            writer.write_background()?;
        }

        Ok(())
    }

    /// Write a plugin call message. Returns the writer for the stream, and the receiver for
    /// messages - i.e. response and engine calls - related to the plugin call
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
                mut call,
                input,
            }) => {
                verify_call_args(&mut call, &self.state.source)?;
                let (header, writer) = self.init_write_pipeline_data(input)?;
                (
                    PluginCall::Run(CallInfo {
                        name,
                        call,
                        input: header,
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
                    sender: Some(tx),
                    context,
                    remaining_streams_to_read: 0,
                },
            ))
            .map_err(|_| ShellError::GenericError {
                error: format!("Plugin `{}` closed unexpectedly", self.state.source.name()),
                msg: "can't complete this operation because the plugin is closed".into(),
                span: match &call {
                    PluginCall::CustomValueOp(value, _) => Some(value.span),
                    PluginCall::Run(info) => Some(info.call.head),
                    _ => None,
                },
                help: Some(format!(
                    "the plugin may have experienced an error. Try registering the plugin again \
                        with `{}`",
                    if let Some(shell) = self.state.source.shell() {
                        format!(
                            "register --shell '{}' '{}'",
                            shell.display(),
                            self.state.source.filename().display(),
                        )
                    } else {
                        format!("register '{}'", self.state.source.filename().display())
                    }
                )),
                inner: vec![],
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
        context: &Option<Context>,
    ) -> Result<PluginCallResponse<PipelineData>, ShellError> {
        // Handle message from receiver
        for msg in rx {
            match msg {
                ReceivedPluginCallMessage::Response(resp) => {
                    return Ok(resp);
                }
                ReceivedPluginCallMessage::Error(err) => {
                    return Err(err);
                }
                ReceivedPluginCallMessage::EngineCall(engine_call_id, engine_call) => {
                    self.handle_engine_call(engine_call_id, engine_call, context)?;
                }
            }
        }
        // If we fail to get a response
        Err(ShellError::PluginFailedToDecode {
            msg: "Failed to receive response to plugin call".into(),
        })
    }

    /// Handle an engine call and write the response.
    fn handle_engine_call(
        &self,
        engine_call_id: EngineCallId,
        engine_call: EngineCall<PipelineData>,
        context: &Option<Context>,
    ) -> Result<(), ShellError> {
        let resp =
            handle_engine_call(engine_call, context).unwrap_or_else(EngineCallResponse::Error);
        // Handle stream
        let (resp, writer) = match resp {
            EngineCallResponse::Error(error) => (EngineCallResponse::Error(error), None),
            EngineCallResponse::Config(config) => (EngineCallResponse::Config(config), None),
            EngineCallResponse::ValueMap(map) => (EngineCallResponse::ValueMap(map), None),
            EngineCallResponse::PipelineData(data) => {
                match self.init_write_pipeline_data(data) {
                    Ok((header, writer)) => {
                        (EngineCallResponse::PipelineData(header), Some(writer))
                    }
                    // just respond with the error if we fail to set it up
                    Err(err) => (EngineCallResponse::Error(err), None),
                }
            }
        };
        // Write the response, then the stream
        self.write(PluginInput::EngineCallResponse(engine_call_id, resp))?;
        self.flush()?;
        if let Some(writer) = writer {
            writer.write_background()?;
        }
        Ok(())
    }

    /// Perform a plugin call. Input and output streams are handled, and engine calls are handled
    /// too if there are any before the final response.
    fn plugin_call(
        &self,
        call: PluginCall<PipelineData>,
        context: &Option<Context>,
    ) -> Result<PluginCallResponse<PipelineData>, ShellError> {
        // Check for an error in the state first, and return it if set.
        if let Some(error) = self.state.error.get() {
            return Err(error.clone());
        }

        // Starting a plugin call adds a lock on the GC. Locks are not added for streams being read
        // by the plugin, so the plugin would have to explicitly tell us if it expects to stay alive
        // while reading streams in the background after the response ends.
        if let Some(ref gc) = self.gc {
            gc.increment_locks(1);
        }

        let (writer, rx) = self.write_plugin_call(call, context.clone())?;

        // Finish writing stream in the background
        writer.write_background()?;

        self.receive_plugin_call_response(rx, context)
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

    /// Do a custom value op that expects a value response (i.e. most of them)
    fn custom_value_op_expecting_value(
        &self,
        value: Spanned<PluginCustomValue>,
        op: CustomValueOp,
    ) -> Result<Value, ShellError> {
        let op_name = op.name();
        let span = value.span;
        let call = PluginCall::CustomValueOp(value, op);
        match self.plugin_call(call, &None)? {
            PluginCallResponse::PipelineData(out_data) => Ok(out_data.into_value(span)),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: format!("Received unexpected response to custom value {op_name}() call"),
            }),
        }
    }

    /// Collapse a custom value to its base value.
    pub(crate) fn custom_value_to_base_value(
        &self,
        value: Spanned<PluginCustomValue>,
    ) -> Result<Value, ShellError> {
        self.custom_value_op_expecting_value(value, CustomValueOp::ToBaseValue)
    }

    /// Follow a numbered cell path on a custom value - e.g. `value.0`.
    pub(crate) fn custom_value_follow_path_int(
        &self,
        value: Spanned<PluginCustomValue>,
        index: Spanned<usize>,
    ) -> Result<Value, ShellError> {
        self.custom_value_op_expecting_value(value, CustomValueOp::FollowPathInt(index))
    }

    /// Follow a named cell path on a custom value - e.g. `value.column`.
    pub(crate) fn custom_value_follow_path_string(
        &self,
        value: Spanned<PluginCustomValue>,
        column_name: Spanned<String>,
    ) -> Result<Value, ShellError> {
        self.custom_value_op_expecting_value(value, CustomValueOp::FollowPathString(column_name))
    }

    /// Invoke comparison logic for custom values.
    pub(crate) fn custom_value_partial_cmp(
        &self,
        value: PluginCustomValue,
        mut other_value: Value,
    ) -> Result<Option<Ordering>, ShellError> {
        PluginCustomValue::verify_source(&mut other_value, &self.state.source)?;
        // Note: the protocol is always designed to have a span with the custom value, but this
        // operation doesn't support one.
        let call = PluginCall::CustomValueOp(
            value.into_spanned(Span::unknown()),
            CustomValueOp::PartialCmp(other_value),
        );
        match self.plugin_call(call, &None)? {
            PluginCallResponse::Ordering(ordering) => Ok(ordering),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response to custom value partial_cmp() call".into(),
            }),
        }
    }

    /// Invoke functionality for an operator on a custom value.
    pub(crate) fn custom_value_operation(
        &self,
        left: Spanned<PluginCustomValue>,
        operator: Spanned<Operator>,
        mut right: Value,
    ) -> Result<Value, ShellError> {
        PluginCustomValue::verify_source(&mut right, &self.state.source)?;
        self.custom_value_op_expecting_value(left, CustomValueOp::Operation(operator, right))
    }

    /// Notify the plugin about a dropped custom value.
    pub(crate) fn custom_value_dropped(&self, value: PluginCustomValue) -> Result<(), ShellError> {
        // Note: the protocol is always designed to have a span with the custom value, but this
        // operation doesn't support one.
        self.custom_value_op_expecting_value(
            value.into_spanned(Span::unknown()),
            CustomValueOp::Dropped,
        )
        .map(|_| ())
    }
}

/// Check that custom values in call arguments come from the right source
fn verify_call_args(
    call: &mut crate::EvaluatedCall,
    source: &Arc<PluginSource>,
) -> Result<(), ShellError> {
    for arg in call.positional.iter_mut() {
        PluginCustomValue::verify_source(arg, source)?;
    }
    for arg in call.named.iter_mut().flat_map(|(_, arg)| arg.as_mut()) {
        PluginCustomValue::verify_source(arg, source)?;
    }
    Ok(())
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
                PluginCustomValue::verify_source(&mut value, &self.state.source)?;
                Ok(PipelineData::Value(value, meta))
            }
            PipelineData::ListStream(ListStream { stream, ctrlc, .. }, meta) => {
                let source = self.state.source.clone();
                Ok(stream
                    .map(move |mut value| {
                        match PluginCustomValue::verify_source(&mut value, &source) {
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

/// Handle an engine call.
pub(crate) fn handle_engine_call(
    call: EngineCall<PipelineData>,
    context: &Option<Context>,
) -> Result<EngineCallResponse<PipelineData>, ShellError> {
    let call_name = call.name();
    let require_context = || {
        context.as_ref().ok_or_else(|| ShellError::GenericError {
            error: "A plugin execution context is required for this engine call".into(),
            msg: format!(
                "attempted to call {} outside of a command invocation",
                call_name
            ),
            span: None,
            help: Some("this is probably a bug with the plugin".into()),
            inner: vec![],
        })
    };
    match call {
        EngineCall::GetConfig => {
            let context = require_context()?;
            let config = Box::new(context.get_config()?);
            Ok(EngineCallResponse::Config(config))
        }
        EngineCall::GetPluginConfig => {
            let context = require_context()?;
            let plugin_config = context.get_plugin_config()?;
            Ok(plugin_config.map_or_else(EngineCallResponse::empty, EngineCallResponse::value))
        }
        EngineCall::GetEnvVar(name) => {
            let context = require_context()?;
            let value = context.get_env_var(&name)?;
            Ok(value.map_or_else(EngineCallResponse::empty, EngineCallResponse::value))
        }
        EngineCall::GetEnvVars => {
            let context = require_context()?;
            context.get_env_vars().map(EngineCallResponse::ValueMap)
        }
        EngineCall::GetCurrentDir => {
            let context = require_context()?;
            let current_dir = context.get_current_dir()?;
            Ok(EngineCallResponse::value(Value::string(
                current_dir.item,
                current_dir.span,
            )))
        }
        EngineCall::EvalClosure {
            closure,
            positional,
            input,
            redirect_stdout,
            redirect_stderr,
        } => require_context()?
            .eval_closure(closure, positional, input, redirect_stdout, redirect_stderr)
            .map(EngineCallResponse::PipelineData),
    }
}
