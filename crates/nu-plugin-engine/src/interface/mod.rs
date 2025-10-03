//! Interface used by the engine to communicate with the plugin.

use nu_plugin_core::{
    Interface, InterfaceManager, PipelineDataWriter, PluginRead, PluginWrite, StreamManager,
    StreamManagerHandle,
    util::{Waitable, WaitableMut, with_custom_values_in},
};
use nu_plugin_protocol::{
    CallInfo, CustomValueOp, EngineCall, EngineCallId, EngineCallResponse, EvaluatedCall, Ordering,
    PluginCall, PluginCallId, PluginCallResponse, PluginCustomValue, PluginInput, PluginOption,
    PluginOutput, ProtocolInfo, StreamId, StreamMessage,
};
use nu_protocol::{
    CustomValue, IntoSpanned, PipelineData, PluginMetadata, PluginSignature, ShellError,
    SignalAction, Signals, Span, Spanned, Value, ast::Operator, casing::Casing, engine::Sequence,
};
use nu_utils::SharedCow;
use std::{
    collections::{BTreeMap, btree_map},
    path::Path,
    sync::{Arc, OnceLock, mpsc},
};

use crate::{
    PluginCustomValueWithSource, PluginExecutionContext, PluginGc, PluginSource,
    process::PluginProcess,
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
pub(crate) struct Context(Box<dyn PluginExecutionContext>);

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
    /// The plugin process being managed
    process: Option<PluginProcess>,
    /// Protocol version info, set after `Hello` received
    protocol_info: Waitable<Arc<ProtocolInfo>>,
    /// Sequence for generating plugin call ids
    plugin_call_id_sequence: Sequence,
    /// Sequence for generating stream ids
    stream_id_sequence: Sequence,
    /// Sender to subscribe to a plugin call response
    plugin_call_subscription_sender: mpsc::Sender<(PluginCallId, PluginCallState)>,
    /// An error that should be propagated to further plugin calls
    error: OnceLock<ShellError>,
    /// The synchronized output writer
    writer: Box<dyn PluginWrite<PluginInput>>,
}

impl std::fmt::Debug for PluginInterfaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginInterfaceState")
            .field("source", &self.source)
            .field("protocol_info", &self.protocol_info)
            .field("plugin_call_id_sequence", &self.plugin_call_id_sequence)
            .field("stream_id_sequence", &self.stream_id_sequence)
            .field(
                "plugin_call_subscription_sender",
                &self.plugin_call_subscription_sender,
            )
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

/// State that the manager keeps for each plugin call during its lifetime.
#[derive(Debug)]
struct PluginCallState {
    /// The sender back to the thread that is waiting for the plugin call response
    sender: Option<mpsc::Sender<ReceivedPluginCallMessage>>,
    /// Don't try to send the plugin call response. This is only used for `Dropped` to avoid an
    /// error
    dont_send_response: bool,
    /// Signals to be used for stream iterators
    signals: Signals,
    /// Channel to receive context on to be used if needed
    context_rx: Option<mpsc::Receiver<Context>>,
    /// Span associated with the call, if any
    span: Option<Span>,
    /// Channel for plugin custom values that should be kept alive for the duration of the plugin
    /// call. The plugin custom values on this channel are never read, we just hold on to it to keep
    /// them in memory so they can be dropped at the end of the call. We hold the sender as well so
    /// we can generate the CurrentCallState.
    keep_plugin_custom_values: (
        mpsc::Sender<PluginCustomValueWithSource>,
        mpsc::Receiver<PluginCustomValueWithSource>,
    ),
    /// Number of streams that still need to be read from the plugin call response
    remaining_streams_to_read: i32,
}

impl Drop for PluginCallState {
    fn drop(&mut self) {
        // Clear the keep custom values channel, so drop notifications can be sent
        for value in self.keep_plugin_custom_values.1.try_iter() {
            log::trace!("Dropping custom value that was kept: {value:?}");
            drop(value);
        }
    }
}

/// Manages reading and dispatching messages for [`PluginInterface`]s.
#[derive(Debug)]
pub struct PluginInterfaceManager {
    /// Shared state
    state: Arc<PluginInterfaceState>,
    /// The writer for protocol info
    protocol_info_mut: WaitableMut<Arc<ProtocolInfo>>,
    /// Manages stream messages and state
    stream_manager: StreamManager,
    /// State related to plugin calls
    plugin_call_states: BTreeMap<PluginCallId, PluginCallState>,
    /// Receiver for plugin call subscriptions
    plugin_call_subscription_receiver: mpsc::Receiver<(PluginCallId, PluginCallState)>,
    /// Tracker for which plugin call streams being read belong to
    ///
    /// This is necessary so we know when we can remove context for plugin calls
    plugin_call_input_streams: BTreeMap<StreamId, PluginCallId>,
    /// Garbage collector handle, to notify about the state of the plugin
    gc: Option<PluginGc>,
}

impl PluginInterfaceManager {
    pub fn new(
        source: Arc<PluginSource>,
        pid: Option<u32>,
        writer: impl PluginWrite<PluginInput> + 'static,
    ) -> PluginInterfaceManager {
        let (subscription_tx, subscription_rx) = mpsc::channel();
        let protocol_info_mut = WaitableMut::new();

        PluginInterfaceManager {
            state: Arc::new(PluginInterfaceState {
                source,
                process: pid.map(PluginProcess::new),
                protocol_info: protocol_info_mut.reader(),
                plugin_call_id_sequence: Sequence::default(),
                stream_id_sequence: Sequence::default(),
                plugin_call_subscription_sender: subscription_tx,
                error: OnceLock::new(),
                writer: Box::new(writer),
            }),
            protocol_info_mut,
            stream_manager: StreamManager::new(),
            plugin_call_states: BTreeMap::new(),
            plugin_call_subscription_receiver: subscription_rx,
            plugin_call_input_streams: BTreeMap::new(),
            gc: None,
        }
    }

    /// Add a garbage collector to this plugin. The manager will notify the garbage collector about
    /// the state of the plugin so that it can be automatically cleaned up if the plugin is
    /// inactive.
    pub fn set_garbage_collector(&mut self, gc: Option<PluginGc>) {
        self.gc = gc;
    }

    /// Consume pending messages in the `plugin_call_subscription_receiver`
    fn receive_plugin_call_subscriptions(&mut self) {
        while let Ok((id, state)) = self.plugin_call_subscription_receiver.try_recv() {
            if let btree_map::Entry::Vacant(e) = self.plugin_call_states.entry(id) {
                e.insert(state);
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
        if let Some(state) = self.plugin_call_states.get_mut(&call_id) {
            state.remaining_streams_to_read += 1;
        }
        // Add a lock to the garbage collector for each stream
        if let Some(ref gc) = self.gc {
            gc.increment_locks(1);
        }
    }

    /// Track the end of an incoming stream
    fn recv_stream_ended(&mut self, stream_id: StreamId) {
        if let Some(call_id) = self.plugin_call_input_streams.remove(&stream_id) {
            if let btree_map::Entry::Occupied(mut e) = self.plugin_call_states.entry(call_id) {
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

    /// Find the [`Signals`] struct corresponding to the given plugin call id
    fn get_signals(&mut self, id: PluginCallId) -> Result<Signals, ShellError> {
        // Make sure we're up to date
        self.receive_plugin_call_subscriptions();
        // Find the subscription and return the context
        self.plugin_call_states
            .get(&id)
            .map(|state| state.signals.clone())
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

        if let btree_map::Entry::Occupied(mut e) = self.plugin_call_states.entry(id) {
            // Remove the subscription sender, since this will be the last message.
            //
            // We can spawn a new one if we need it for engine calls.
            if !e.get().dont_send_response
                && e.get_mut()
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

        if let Some(state) = self.plugin_call_states.get_mut(&id) {
            if state.sender.is_none() {
                let (tx, rx) = mpsc::channel();
                let context_rx =
                    state
                        .context_rx
                        .take()
                        .ok_or_else(|| ShellError::NushellFailed {
                            msg: "Tried to spawn the fallback engine call handler more than once"
                                .into(),
                        })?;

                // Generate the state needed to handle engine calls
                let mut current_call_state = CurrentCallState {
                    context_tx: None,
                    keep_plugin_custom_values_tx: Some(state.keep_plugin_custom_values.0.clone()),
                    entered_foreground: false,
                    span: state.span,
                };

                let handler = move || {
                    // We receive on the thread so that we don't block the reader thread
                    let mut context = context_rx
                        .recv()
                        .ok() // The plugin call won't send context if it's not required.
                        .map(|c| c.0);

                    for msg in rx {
                        // This thread only handles engine calls.
                        match msg {
                            ReceivedPluginCallMessage::EngineCall(engine_call_id, engine_call) => {
                                if let Err(err) = interface.handle_engine_call(
                                    engine_call_id,
                                    engine_call,
                                    &mut current_call_state,
                                    context.as_deref_mut(),
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
                state.sender = Some(tx);
                Ok(state.sender.as_ref().unwrap_or_else(|| unreachable!()))
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
        if let Some(subscription) = self.plugin_call_states.get(&plugin_call_id) {
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
                    EngineCallResponse::Error(ShellError::GenericError {
                        error: "Caller hung up".to_string(),
                        msg: "Can't make engine call because the original caller hung up"
                            .to_string(),
                        span: None,
                        help: None,
                        inner: vec![],
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
    pub fn is_finished(&self) -> bool {
        Arc::strong_count(&self.state) < 2
    }

    /// Loop on input from the given reader as long as `is_finished()` is false
    ///
    /// Any errors will be propagated to all read streams automatically.
    pub fn consume_all(
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
                for subscription in std::mem::take(&mut self.plugin_call_states).into_values() {
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
        log::trace!("from plugin: {input:?}");

        match input {
            PluginOutput::Hello(info) => {
                let info = Arc::new(info);
                self.protocol_info_mut.set(info.clone())?;

                let local_info = ProtocolInfo::default();
                if local_info.is_compatible_with(&info)? {
                    Ok(())
                } else {
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
            _ if !self.state.protocol_info.is_set() => {
                // Must send protocol info first
                Err(ShellError::PluginFailedToLoad {
                    msg: format!(
                        "Failed to receive initial Hello message from `{}`. \
                            This plugin might be too old",
                        self.state.source.name()
                    ),
                })
            }
            // Stream messages
            PluginOutput::Data(..)
            | PluginOutput::End(..)
            | PluginOutput::Drop(..)
            | PluginOutput::Ack(..) => {
                self.consume_stream_message(input.try_into().map_err(|msg| {
                    ShellError::NushellFailed {
                        msg: format!("Failed to convert message {msg:?} to StreamMessage"),
                    }
                })?)
            }
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
                let response = response
                    .map_data(|data| {
                        let signals = self.get_signals(id)?;

                        // Register the stream in the response
                        if let Some(stream_id) = data.stream_id() {
                            self.recv_stream_started(id, stream_id);
                        }

                        self.read_pipeline_data(data, &signals)
                    })
                    .unwrap_or_else(|err| {
                        // If there's an error with initializing this stream, change it to a plugin
                        // error response, but send it anyway
                        PluginCallResponse::Error(err.into())
                    });
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
                let call = call
                    // Handle reading the pipeline data, if any
                    .map_data(|input| {
                        let signals = self.get_signals(context)?;
                        self.read_pipeline_data(input, &signals)
                    })
                    // Do anything extra needed for each engine call setup
                    .and_then(|mut engine_call| {
                        match engine_call {
                            EngineCall::EvalClosure {
                                ref mut positional, ..
                            } => {
                                for arg in positional.iter_mut() {
                                    // Add source to any plugin custom values in the arguments
                                    PluginCustomValueWithSource::add_source_in(
                                        arg,
                                        &self.state.source,
                                    )?;
                                }
                                Ok(engine_call)
                            }
                            _ => Ok(engine_call),
                        }
                    });
                match call {
                    Ok(call) => self.send_engine_call(context, id, call),
                    // If there was an error with setting up the call, just write the error
                    Err(err) => self.get_interface().write_engine_call_response(
                        id,
                        EngineCallResponse::Error(err),
                        &CurrentCallState::default(),
                    ),
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
                with_custom_values_in(value, |custom_value| {
                    PluginCustomValueWithSource::add_source(custom_value.item, &self.state.source);
                    Ok::<_, ShellError>(())
                })?;
                Ok(data)
            }
            PipelineData::ListStream(stream, meta) => {
                let source = self.state.source.clone();
                Ok(PipelineData::list_stream(
                    stream.map(move |mut value| {
                        let _ = PluginCustomValueWithSource::add_source_in(&mut value, &source);
                        value
                    }),
                    meta,
                ))
            }
            PipelineData::Empty | PipelineData::ByteStream(..) => Ok(data),
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
///
/// This is not a public API.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct PluginInterface {
    /// Shared state
    state: Arc<PluginInterfaceState>,
    /// Handle to stream manager
    stream_manager_handle: StreamManagerHandle,
    /// Handle to plugin garbage collector
    gc: Option<PluginGc>,
}

impl PluginInterface {
    /// Get the process ID for the plugin, if known.
    pub fn pid(&self) -> Option<u32> {
        self.state.process.as_ref().map(|p| p.pid())
    }

    /// Get the protocol info for the plugin. Will block to receive `Hello` if not received yet.
    pub fn protocol_info(&self) -> Result<Arc<ProtocolInfo>, ShellError> {
        self.state.protocol_info.get().and_then(|info| {
            info.ok_or_else(|| ShellError::PluginFailedToLoad {
                msg: format!(
                    "Failed to get protocol info (`Hello` message) from the `{}` plugin",
                    self.state.source.identity.name()
                ),
            })
        })
    }

    /// Write the protocol info. This should be done after initialization
    pub fn hello(&self) -> Result<(), ShellError> {
        self.write(PluginInput::Hello(ProtocolInfo::default()))?;
        self.flush()
    }

    /// Tell the plugin it should not expect any more plugin calls and should terminate after it has
    /// finished processing the ones it has already received.
    ///
    /// Note that this is automatically called when the last existing `PluginInterface` is dropped.
    /// You probably do not need to call this manually.
    pub fn goodbye(&self) -> Result<(), ShellError> {
        self.write(PluginInput::Goodbye)?;
        self.flush()
    }

    /// Send the plugin a signal.
    pub fn signal(&self, action: SignalAction) -> Result<(), ShellError> {
        self.write(PluginInput::Signal(action))?;
        self.flush()
    }

    /// Write an [`EngineCallResponse`]. Writes the full stream contained in any [`PipelineData`]
    /// before returning.
    pub fn write_engine_call_response(
        &self,
        id: EngineCallId,
        response: EngineCallResponse<PipelineData>,
        state: &CurrentCallState,
    ) -> Result<(), ShellError> {
        // Set up any stream if necessary
        let mut writer = None;
        let response = response.map_data(|data| {
            let (data_header, data_writer) = self.init_write_pipeline_data(data, state)?;
            writer = Some(data_writer);
            Ok(data_header)
        })?;

        // Write the response, including the pipeline data header if present
        self.write(PluginInput::EngineCallResponse(id, response))?;
        self.flush()?;

        // If we have a stream to write, do it now
        if let Some(writer) = writer {
            writer.write_background()?;
        }

        Ok(())
    }

    /// Write a plugin call message. Returns the writer for the stream.
    fn write_plugin_call(
        &self,
        mut call: PluginCall<PipelineData>,
        context: Option<&dyn PluginExecutionContext>,
    ) -> Result<WritePluginCallResult, ShellError> {
        let id = self.state.plugin_call_id_sequence.next()?;
        let signals = context
            .map(|c| c.signals().clone())
            .unwrap_or_else(Signals::empty);
        let (tx, rx) = mpsc::channel();
        let (context_tx, context_rx) = mpsc::channel();
        let keep_plugin_custom_values = mpsc::channel();

        // Set up the state that will stay alive during the call.
        let state = CurrentCallState {
            context_tx: Some(context_tx),
            keep_plugin_custom_values_tx: Some(keep_plugin_custom_values.0.clone()),
            entered_foreground: false,
            span: call.span(),
        };

        // Prepare the call with the state.
        state.prepare_plugin_call(&mut call, &self.state.source)?;

        // Convert the call into one with a header and handle the stream, if necessary
        let (call, writer) = match call {
            PluginCall::Metadata => (PluginCall::Metadata, Default::default()),
            PluginCall::Signature => (PluginCall::Signature, Default::default()),
            PluginCall::CustomValueOp(value, op) => {
                (PluginCall::CustomValueOp(value, op), Default::default())
            }
            PluginCall::Run(CallInfo { name, call, input }) => {
                let (header, writer) = self.init_write_pipeline_data(input, &state)?;
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

        // Don't try to send a response for a Dropped call.
        let dont_send_response =
            matches!(call, PluginCall::CustomValueOp(_, CustomValueOp::Dropped));

        // Register the subscription to the response, and the context
        self.state
            .plugin_call_subscription_sender
            .send((
                id,
                PluginCallState {
                    sender: Some(tx).filter(|_| !dont_send_response),
                    dont_send_response,
                    signals,
                    context_rx: Some(context_rx),
                    span: call.span(),
                    keep_plugin_custom_values,
                    remaining_streams_to_read: 0,
                },
            ))
            .map_err(|_| {
                let existing_error = self.state.error.get().cloned();
                ShellError::GenericError {
                    error: format!("Plugin `{}` closed unexpectedly", self.state.source.name()),
                    msg: "can't complete this operation because the plugin is closed".into(),
                    span: call.span(),
                    help: Some(format!(
                        "the plugin may have experienced an error. Try loading the plugin again \
                        with `{}`",
                        self.state.source.identity.use_command(),
                    )),
                    inner: existing_error.into_iter().collect(),
                }
            })?;

        // Starting a plugin call adds a lock on the GC. Locks are not added for streams being read
        // by the plugin, so the plugin would have to explicitly tell us if it expects to stay alive
        // while reading streams in the background after the response ends.
        if let Some(ref gc) = self.gc {
            gc.increment_locks(1);
        }

        // Write request
        self.write(PluginInput::Call(id, call))?;
        self.flush()?;

        Ok(WritePluginCallResult {
            receiver: rx,
            writer,
            state,
        })
    }

    /// Read the channel for plugin call messages and handle them until the response is received.
    fn receive_plugin_call_response(
        &self,
        rx: mpsc::Receiver<ReceivedPluginCallMessage>,
        mut context: Option<&mut (dyn PluginExecutionContext + '_)>,
        mut state: CurrentCallState,
    ) -> Result<PluginCallResponse<PipelineData>, ShellError> {
        // Handle message from receiver
        for msg in rx {
            match msg {
                ReceivedPluginCallMessage::Response(resp) => {
                    if state.entered_foreground {
                        // Make the plugin leave the foreground on return, even if it's a stream
                        if let Some(context) = context.as_deref_mut()
                            && let Err(err) =
                                set_foreground(self.state.process.as_ref(), context, false)
                        {
                            log::warn!("Failed to leave foreground state on exit: {err:?}");
                        }
                    }
                    if resp.has_stream() {
                        // If the response has a stream, we need to register the context
                        if let Some(context) = context
                            && let Some(ref context_tx) = state.context_tx
                        {
                            let _ = context_tx.send(Context(context.boxed()));
                        }
                    }
                    return Ok(resp);
                }
                ReceivedPluginCallMessage::Error(err) => {
                    return Err(err);
                }
                ReceivedPluginCallMessage::EngineCall(engine_call_id, engine_call) => {
                    self.handle_engine_call(
                        engine_call_id,
                        engine_call,
                        &mut state,
                        context.as_deref_mut(),
                    )?;
                }
            }
        }
        // If we fail to get a response, check for an error in the state first, and return it if
        // set. This is probably a much more helpful error than 'failed to receive response' alone
        let existing_error = self.state.error.get().cloned();
        Err(ShellError::GenericError {
            error: format!(
                "Failed to receive response to plugin call from `{}`",
                self.state.source.identity.name()
            ),
            msg: "while waiting for this operation to complete".into(),
            span: state.span,
            help: Some(format!(
                "try restarting the plugin with `{}`",
                self.state.source.identity.use_command()
            )),
            inner: existing_error.into_iter().collect(),
        })
    }

    /// Handle an engine call and write the response.
    fn handle_engine_call(
        &self,
        engine_call_id: EngineCallId,
        engine_call: EngineCall<PipelineData>,
        state: &mut CurrentCallState,
        context: Option<&mut (dyn PluginExecutionContext + '_)>,
    ) -> Result<(), ShellError> {
        let process = self.state.process.as_ref();
        let resp = handle_engine_call(engine_call, state, context, process)
            .unwrap_or_else(EngineCallResponse::Error);
        // Handle stream
        let mut writer = None;
        let resp = resp
            .map_data(|data| {
                let (data_header, data_writer) = self.init_write_pipeline_data(data, state)?;
                writer = Some(data_writer);
                Ok(data_header)
            })
            .unwrap_or_else(|err| {
                // If we fail to set up the response write, change to an error response here
                writer = None;
                EngineCallResponse::Error(err)
            });
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
        context: Option<&mut dyn PluginExecutionContext>,
    ) -> Result<PluginCallResponse<PipelineData>, ShellError> {
        // Check for an error in the state first, and return it if set.
        if let Some(error) = self.state.error.get() {
            return Err(ShellError::GenericError {
                error: format!(
                    "Failed to send plugin call to `{}`",
                    self.state.source.identity.name()
                ),
                msg: "the plugin encountered an error before this operation could be attempted"
                    .into(),
                span: call.span(),
                help: Some(format!(
                    "try loading the plugin again with `{}`",
                    self.state.source.identity.use_command(),
                )),
                inner: vec![error.clone()],
            });
        }

        let result = self.write_plugin_call(call, context.as_deref())?;

        // Finish writing stream in the background
        result.writer.write_background()?;

        self.receive_plugin_call_response(result.receiver, context, result.state)
    }

    /// Get the metadata from the plugin.
    pub fn get_metadata(&self) -> Result<PluginMetadata, ShellError> {
        match self.plugin_call(PluginCall::Metadata, None)? {
            PluginCallResponse::Metadata(meta) => Ok(meta),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response to plugin Metadata call".into(),
            }),
        }
    }

    /// Get the command signatures from the plugin.
    pub fn get_signature(&self) -> Result<Vec<PluginSignature>, ShellError> {
        match self.plugin_call(PluginCall::Signature, None)? {
            PluginCallResponse::Signature(sigs) => Ok(sigs),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response to plugin Signature call".into(),
            }),
        }
    }

    /// Run the plugin with the given call and execution context.
    pub fn run(
        &self,
        call: CallInfo<PipelineData>,
        context: &mut dyn PluginExecutionContext,
    ) -> Result<PipelineData, ShellError> {
        match self.plugin_call(PluginCall::Run(call), Some(context))? {
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
        value: Spanned<PluginCustomValueWithSource>,
        op: CustomValueOp,
    ) -> Result<Value, ShellError> {
        let op_name = op.name();
        let span = value.span;

        // Check that the value came from the right source
        value.item.verify_source(span, &self.state.source)?;

        let call = PluginCall::CustomValueOp(value.map(|cv| cv.without_source()), op);
        match self.plugin_call(call, None)? {
            PluginCallResponse::PipelineData(out_data) => out_data.into_value(span),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: format!("Received unexpected response to custom value {op_name}() call"),
            }),
        }
    }

    /// Collapse a custom value to its base value.
    pub fn custom_value_to_base_value(
        &self,
        value: Spanned<PluginCustomValueWithSource>,
    ) -> Result<Value, ShellError> {
        self.custom_value_op_expecting_value(value, CustomValueOp::ToBaseValue)
    }

    /// Follow a numbered cell path on a custom value - e.g. `value.0`.
    pub fn custom_value_follow_path_int(
        &self,
        value: Spanned<PluginCustomValueWithSource>,
        index: Spanned<usize>,
        optional: bool,
    ) -> Result<Value, ShellError> {
        self.custom_value_op_expecting_value(
            value,
            CustomValueOp::FollowPathInt { index, optional },
        )
    }

    /// Follow a named cell path on a custom value - e.g. `value.column`.
    pub fn custom_value_follow_path_string(
        &self,
        value: Spanned<PluginCustomValueWithSource>,
        column_name: Spanned<String>,
        optional: bool,
        casing: Casing,
    ) -> Result<Value, ShellError> {
        self.custom_value_op_expecting_value(
            value,
            CustomValueOp::FollowPathString {
                column_name,
                optional,
                casing,
            },
        )
    }

    /// Invoke comparison logic for custom values.
    pub fn custom_value_partial_cmp(
        &self,
        value: PluginCustomValueWithSource,
        other_value: Value,
    ) -> Result<Option<Ordering>, ShellError> {
        // Check that the value came from the right source
        value.verify_source(Span::unknown(), &self.state.source)?;

        // Note: the protocol is always designed to have a span with the custom value, but this
        // operation doesn't support one.
        let call = PluginCall::CustomValueOp(
            value.without_source().into_spanned(Span::unknown()),
            CustomValueOp::PartialCmp(other_value),
        );
        match self.plugin_call(call, None)? {
            PluginCallResponse::Ordering(ordering) => Ok(ordering),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response to custom value partial_cmp() call".into(),
            }),
        }
    }

    /// Invoke functionality for an operator on a custom value.
    pub fn custom_value_operation(
        &self,
        left: Spanned<PluginCustomValueWithSource>,
        operator: Spanned<Operator>,
        right: Value,
    ) -> Result<Value, ShellError> {
        self.custom_value_op_expecting_value(left, CustomValueOp::Operation(operator, right))
    }

    /// Invoke saving operation on a custom value.
    pub fn custom_value_save(
        &self,
        value: Spanned<PluginCustomValueWithSource>,
        path: Spanned<&Path>,
        save_call_span: Span,
    ) -> Result<(), ShellError> {
        // Check that the value came from the right source
        value.item.verify_source(value.span, &self.state.source)?;

        let call = PluginCall::CustomValueOp(
            value.map(|cv| cv.without_source()),
            CustomValueOp::Save {
                path: path.map(ToOwned::to_owned),
                save_call_span,
            },
        );
        match self.plugin_call(call, None)? {
            PluginCallResponse::Ok => Ok(()),
            PluginCallResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response to custom value save() call".into(),
            }),
        }
    }

    /// Notify the plugin about a dropped custom value.
    pub fn custom_value_dropped(&self, value: PluginCustomValue) -> Result<(), ShellError> {
        // Make sure we don't block here. This can happen on the receiver thread, which would cause a deadlock. We should not try to receive the response - just let it be discarded.
        //
        // Note: the protocol is always designed to have a span with the custom value, but this
        // operation doesn't support one.
        drop(self.write_plugin_call(
            PluginCall::CustomValueOp(value.into_spanned(Span::unknown()), CustomValueOp::Dropped),
            None,
        )?);
        Ok(())
    }
}

impl Interface for PluginInterface {
    type Output = PluginInput;
    type DataContext = CurrentCallState;

    fn write(&self, input: PluginInput) -> Result<(), ShellError> {
        log::trace!("to plugin: {input:?}");
        self.state.writer.write(&input).map_err(|err| {
            log::warn!("write() error: {err}");
            // If there's an error in the state, return that instead because it's likely more
            // descriptive
            self.state.error.get().cloned().unwrap_or(err)
        })
    }

    fn flush(&self) -> Result<(), ShellError> {
        self.state.writer.flush().map_err(|err| {
            log::warn!("flush() error: {err}");
            // If there's an error in the state, return that instead because it's likely more
            // descriptive
            self.state.error.get().cloned().unwrap_or(err)
        })
    }

    fn stream_id_sequence(&self) -> &Sequence {
        &self.state.stream_id_sequence
    }

    fn stream_manager_handle(&self) -> &StreamManagerHandle {
        &self.stream_manager_handle
    }

    fn prepare_pipeline_data(
        &self,
        data: PipelineData,
        state: &CurrentCallState,
    ) -> Result<PipelineData, ShellError> {
        // Validate the destination of values in the pipeline data
        match data {
            PipelineData::Value(mut value, meta) => {
                state.prepare_value(&mut value, &self.state.source)?;
                Ok(PipelineData::value(value, meta))
            }
            PipelineData::ListStream(stream, meta) => {
                let source = self.state.source.clone();
                let state = state.clone();
                Ok(PipelineData::list_stream(
                    stream.map(move |mut value| {
                        match state.prepare_value(&mut value, &source) {
                            Ok(()) => value,
                            // Put the error in the stream instead
                            Err(err) => Value::error(err, value.span()),
                        }
                    }),
                    meta,
                ))
            }
            PipelineData::Empty | PipelineData::ByteStream(..) => Ok(data),
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
        if Arc::strong_count(&self.state) < 3
            && let Err(err) = self.goodbye()
        {
            log::warn!("Error during plugin Goodbye: {err}");
        }
    }
}

/// Return value of [`PluginInterface::write_plugin_call()`].
#[must_use]
struct WritePluginCallResult {
    /// Receiver for plugin call messages related to the written plugin call.
    receiver: mpsc::Receiver<ReceivedPluginCallMessage>,
    /// Writer for the stream, if any.
    writer: PipelineDataWriter<PluginInterface>,
    /// State to be kept for the duration of the plugin call.
    state: CurrentCallState,
}

/// State related to the current plugin call being executed.
#[derive(Default, Clone)]
pub struct CurrentCallState {
    /// Sender for context, which should be sent if the plugin call returned a stream so that
    /// engine calls may continue to be handled.
    context_tx: Option<mpsc::Sender<Context>>,
    /// Sender for a channel that retains plugin custom values that need to stay alive for the
    /// duration of a plugin call.
    keep_plugin_custom_values_tx: Option<mpsc::Sender<PluginCustomValueWithSource>>,
    /// The plugin call entered the foreground: this should be cleaned up automatically when the
    /// plugin call returns.
    entered_foreground: bool,
    /// The span that caused the plugin call.
    span: Option<Span>,
}

impl CurrentCallState {
    /// Prepare a custom value for write. Verifies custom value origin, and keeps custom values that
    /// shouldn't be dropped immediately.
    fn prepare_custom_value(
        &self,
        custom_value: Spanned<&mut Box<dyn CustomValue>>,
        source: &PluginSource,
    ) -> Result<(), ShellError> {
        // Ensure we can use it
        PluginCustomValueWithSource::verify_source_of_custom_value(
            custom_value.as_deref().map(|cv| &**cv),
            source,
        )?;

        // Check whether we need to keep it
        if let Some(keep_tx) = &self.keep_plugin_custom_values_tx
            && let Some(custom_value) = custom_value
                .item
                .as_any()
                .downcast_ref::<PluginCustomValueWithSource>()
            && custom_value.notify_on_drop()
        {
            log::trace!("Keeping custom value for drop later: {custom_value:?}");
            keep_tx
                .send(custom_value.clone())
                .map_err(|_| ShellError::NushellFailed {
                    msg: "Failed to custom value to keep channel".into(),
                })?;
        }

        // Strip the source from it so it can be serialized
        PluginCustomValueWithSource::remove_source(&mut *custom_value.item);

        Ok(())
    }

    /// Prepare a value for write, including all contained custom values.
    fn prepare_value(&self, value: &mut Value, source: &PluginSource) -> Result<(), ShellError> {
        with_custom_values_in(value, |custom_value| {
            self.prepare_custom_value(custom_value, source)
        })
    }

    /// Prepare call arguments for write.
    fn prepare_call_args(
        &self,
        call: &mut EvaluatedCall,
        source: &PluginSource,
    ) -> Result<(), ShellError> {
        for arg in call.positional.iter_mut() {
            self.prepare_value(arg, source)?;
        }
        for arg in call.named.iter_mut().flat_map(|(_, arg)| arg.as_mut()) {
            self.prepare_value(arg, source)?;
        }
        Ok(())
    }

    /// Prepare a plugin call for write. Does not affect pipeline data, which is handled by
    /// `prepare_pipeline_data()` instead.
    fn prepare_plugin_call<D>(
        &self,
        call: &mut PluginCall<D>,
        source: &PluginSource,
    ) -> Result<(), ShellError> {
        match call {
            PluginCall::Metadata => Ok(()),
            PluginCall::Signature => Ok(()),
            PluginCall::Run(CallInfo { call, .. }) => self.prepare_call_args(call, source),
            PluginCall::CustomValueOp(_, op) => {
                // Handle anything within the op.
                match op {
                    CustomValueOp::ToBaseValue => Ok(()),
                    CustomValueOp::FollowPathInt { .. } => Ok(()),
                    CustomValueOp::FollowPathString { .. } => Ok(()),
                    CustomValueOp::PartialCmp(value) => self.prepare_value(value, source),
                    CustomValueOp::Operation(_, value) => self.prepare_value(value, source),
                    CustomValueOp::Save { .. } => Ok(()),
                    CustomValueOp::Dropped => Ok(()),
                }
            }
        }
    }
}

/// Handle an engine call.
pub(crate) fn handle_engine_call(
    call: EngineCall<PipelineData>,
    state: &mut CurrentCallState,
    context: Option<&mut (dyn PluginExecutionContext + '_)>,
    process: Option<&PluginProcess>,
) -> Result<EngineCallResponse<PipelineData>, ShellError> {
    let call_name = call.name();

    let context = context.ok_or_else(|| ShellError::GenericError {
        error: "A plugin execution context is required for this engine call".into(),
        msg: format!("attempted to call {call_name} outside of a command invocation"),
        span: None,
        help: Some("this is probably a bug with the plugin".into()),
        inner: vec![],
    })?;

    match call {
        EngineCall::GetConfig => {
            let config = SharedCow::from(context.get_config()?);
            Ok(EngineCallResponse::Config(config))
        }
        EngineCall::GetPluginConfig => {
            let plugin_config = context.get_plugin_config()?;
            Ok(plugin_config.map_or_else(EngineCallResponse::empty, EngineCallResponse::value))
        }
        EngineCall::GetEnvVar(name) => {
            let value = context.get_env_var(&name)?;
            Ok(value
                .cloned()
                .map_or_else(EngineCallResponse::empty, EngineCallResponse::value))
        }
        EngineCall::GetEnvVars => context.get_env_vars().map(EngineCallResponse::ValueMap),
        EngineCall::GetCurrentDir => {
            let current_dir = context.get_current_dir()?;
            Ok(EngineCallResponse::value(Value::string(
                current_dir.item,
                current_dir.span,
            )))
        }
        EngineCall::AddEnvVar(name, value) => {
            context.add_env_var(name, value)?;
            Ok(EngineCallResponse::empty())
        }
        EngineCall::GetHelp => {
            let help = context.get_help()?;
            Ok(EngineCallResponse::value(Value::string(
                help.item, help.span,
            )))
        }
        EngineCall::EnterForeground => {
            let resp = set_foreground(process, context, true)?;
            state.entered_foreground = true;
            Ok(resp)
        }
        EngineCall::LeaveForeground => {
            let resp = set_foreground(process, context, false)?;
            state.entered_foreground = false;
            Ok(resp)
        }
        EngineCall::GetSpanContents(span) => {
            let contents = context.get_span_contents(span)?;
            Ok(EngineCallResponse::value(Value::binary(
                contents.item,
                contents.span,
            )))
        }
        EngineCall::EvalClosure {
            closure,
            positional,
            input,
            redirect_stdout,
            redirect_stderr,
        } => context
            .eval_closure(closure, positional, input, redirect_stdout, redirect_stderr)
            .map(EngineCallResponse::PipelineData),
        EngineCall::FindDecl(name) => context.find_decl(&name).map(|decl_id| {
            if let Some(decl_id) = decl_id {
                EngineCallResponse::Identifier(decl_id)
            } else {
                EngineCallResponse::empty()
            }
        }),
        EngineCall::CallDecl {
            decl_id,
            call,
            input,
            redirect_stdout,
            redirect_stderr,
        } => context
            .call_decl(decl_id, call, input, redirect_stdout, redirect_stderr)
            .map(EngineCallResponse::PipelineData),
    }
}

/// Implements enter/exit foreground
fn set_foreground(
    process: Option<&PluginProcess>,
    context: &mut dyn PluginExecutionContext,
    enter: bool,
) -> Result<EngineCallResponse<PipelineData>, ShellError> {
    if let Some(process) = process {
        if let Some(pipeline_externals_state) = context.pipeline_externals_state() {
            if enter {
                let pgrp = process.enter_foreground(context.span(), pipeline_externals_state)?;
                Ok(pgrp.map_or_else(EngineCallResponse::empty, |id| {
                    EngineCallResponse::value(Value::int(id as i64, context.span()))
                }))
            } else {
                process.exit_foreground()?;
                Ok(EngineCallResponse::empty())
            }
        } else {
            // This should always be present on a real context
            Err(ShellError::NushellFailed {
                msg: "missing required pipeline_externals_state from context \
                            for entering foreground"
                    .into(),
            })
        }
    } else {
        Err(ShellError::GenericError {
            error: "Can't manage plugin process to enter foreground".into(),
            msg: "the process ID for this plugin is unknown".into(),
            span: Some(context.span()),
            help: Some("the plugin may be running in a test".into()),
            inner: vec![],
        })
    }
}
