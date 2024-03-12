//! Interface used by the plugin to communicate with the engine.

use std::{
    collections::{btree_map, BTreeMap, HashMap},
    sync::{mpsc, Arc},
};

use nu_protocol::{
    engine::Closure, Config, IntoInterruptiblePipelineData, ListStream, PipelineData,
    PluginSignature, ShellError, Spanned, Value,
};

use crate::{
    protocol::{
        CallInfo, CustomValueOp, EngineCall, EngineCallId, EngineCallResponse, Ordering,
        PluginCall, PluginCallId, PluginCallResponse, PluginCustomValue, PluginInput, PluginOption,
        ProtocolInfo,
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
    /// Sequence for generating engine call ids
    engine_call_id_sequence: Sequence,
    /// Sequence for generating stream ids
    stream_id_sequence: Sequence,
    /// Sender to subscribe to an engine call response
    engine_call_subscription_sender:
        mpsc::Sender<(EngineCallId, mpsc::Sender<EngineCallResponse<PipelineData>>)>,
    /// The synchronized output writer
    writer: Box<dyn PluginWrite<PluginOutput>>,
}

impl std::fmt::Debug for EngineInterfaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineInterfaceState")
            .field("engine_call_id_sequence", &self.engine_call_id_sequence)
            .field("stream_id_sequence", &self.stream_id_sequence)
            .field(
                "engine_call_subscription_sender",
                &self.engine_call_subscription_sender,
            )
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
    /// Subscriptions for engine call responses
    engine_call_subscriptions:
        BTreeMap<EngineCallId, mpsc::Sender<EngineCallResponse<PipelineData>>>,
    /// Receiver for engine call subscriptions
    engine_call_subscription_receiver:
        mpsc::Receiver<(EngineCallId, mpsc::Sender<EngineCallResponse<PipelineData>>)>,
    /// Manages stream messages and state
    stream_manager: StreamManager,
    /// Protocol version info, set after `Hello` received
    protocol_info: Option<ProtocolInfo>,
}

impl EngineInterfaceManager {
    pub(crate) fn new(writer: impl PluginWrite<PluginOutput> + 'static) -> EngineInterfaceManager {
        let (plug_tx, plug_rx) = mpsc::channel();
        let (subscription_tx, subscription_rx) = mpsc::channel();

        EngineInterfaceManager {
            state: Arc::new(EngineInterfaceState {
                engine_call_id_sequence: Sequence::default(),
                stream_id_sequence: Sequence::default(),
                engine_call_subscription_sender: subscription_tx,
                writer: Box::new(writer),
            }),
            plugin_call_sender: Some(plug_tx),
            plugin_call_receiver: Some(plug_rx),
            engine_call_subscriptions: BTreeMap::new(),
            engine_call_subscription_receiver: subscription_rx,
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

    /// Flush any remaining subscriptions in the receiver into the map
    fn receive_engine_call_subscriptions(&mut self) {
        for (id, subscription) in self.engine_call_subscription_receiver.try_iter() {
            if let btree_map::Entry::Vacant(e) = self.engine_call_subscriptions.entry(id) {
                e.insert(subscription);
            } else {
                log::warn!("Duplicate engine call ID ignored: {id}")
            }
        }
    }

    /// Send a [`EngineCallResponse`] to the appropriate sender
    fn send_engine_call_response(
        &mut self,
        id: EngineCallId,
        response: EngineCallResponse<PipelineData>,
    ) -> Result<(), ShellError> {
        // Ensure all of the subscriptions have been flushed out of the receiver
        self.receive_engine_call_subscriptions();
        // Remove the sender - there is only one response per engine call
        if let Some(sender) = self.engine_call_subscriptions.remove(&id) {
            if sender.send(response).is_err() {
                log::warn!("Received an engine call response for id={id}, but the caller hung up");
            }
            Ok(())
        } else {
            Err(ShellError::PluginFailedToDecode {
                msg: format!("Unknown engine call ID: {id}"),
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
        mut reader: impl PluginRead<PluginInput>,
    ) -> Result<(), ShellError> {
        while let Some(msg) = reader.read().transpose() {
            if self.is_finished() {
                break;
            }

            if let Err(err) = msg.and_then(|msg| self.consume(msg)) {
                // Error to streams
                let _ = self.stream_manager.broadcast_read_error(err.clone());
                // Error to engine call waiters
                self.receive_engine_call_subscriptions();
                for sender in std::mem::take(&mut self.engine_call_subscriptions).into_values() {
                    let _ = sender.send(EngineCallResponse::Error(err.clone()));
                }
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
                                call: CallInfo { name, call, input },
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
            PluginInput::EngineCallResponse(id, response) => {
                let response = match response {
                    EngineCallResponse::Error(err) => EngineCallResponse::Error(err),
                    EngineCallResponse::Config(config) => EngineCallResponse::Config(config),
                    EngineCallResponse::ValueMap(map) => EngineCallResponse::ValueMap(map),
                    EngineCallResponse::PipelineData(header) => {
                        // If there's an error with initializing this stream, change it to an engine
                        // call error response, but send it anyway
                        match self.read_pipeline_data(header, None) {
                            Ok(data) => EngineCallResponse::PipelineData(data),
                            Err(err) => EngineCallResponse::Error(err),
                        }
                    }
                };
                self.send_engine_call_response(id, response)
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

    /// Write an engine call message. Returns the writer for the stream, and the receiver for
    /// the response to the engine call.
    fn write_engine_call(
        &self,
        call: EngineCall<PipelineData>,
    ) -> Result<
        (
            PipelineDataWriter<Self>,
            mpsc::Receiver<EngineCallResponse<PipelineData>>,
        ),
        ShellError,
    > {
        let context = self.context()?;
        let id = self.state.engine_call_id_sequence.next()?;
        let (tx, rx) = mpsc::channel();

        // Convert the call into one with a header and handle the stream, if necessary
        let (call, writer) = match call {
            EngineCall::EvalClosure {
                closure,
                positional,
                input,
                redirect_stdout,
                redirect_stderr,
            } => {
                let (header, writer) = self.init_write_pipeline_data(input)?;
                (
                    EngineCall::EvalClosure {
                        closure,
                        positional,
                        input: header,
                        redirect_stdout,
                        redirect_stderr,
                    },
                    writer,
                )
            }
            // These calls have no pipeline data, so they're just the same on both sides
            EngineCall::GetConfig => (EngineCall::GetConfig, Default::default()),
            EngineCall::GetPluginConfig => (EngineCall::GetPluginConfig, Default::default()),
            EngineCall::GetEnvVar(name) => (EngineCall::GetEnvVar(name), Default::default()),
            EngineCall::GetEnvVars => (EngineCall::GetEnvVars, Default::default()),
            EngineCall::GetCurrentDir => (EngineCall::GetCurrentDir, Default::default()),
        };

        // Register the channel
        self.state
            .engine_call_subscription_sender
            .send((id, tx))
            .map_err(|_| ShellError::NushellFailed {
                msg: "EngineInterfaceManager hung up and is no longer accepting engine calls"
                    .into(),
            })?;

        // Write request
        self.write(PluginOutput::EngineCall { context, id, call })?;
        self.flush()?;

        Ok((writer, rx))
    }

    /// Perform an engine call. Input and output streams are handled.
    fn engine_call(
        &self,
        call: EngineCall<PipelineData>,
    ) -> Result<EngineCallResponse<PipelineData>, ShellError> {
        let (writer, rx) = self.write_engine_call(call)?;

        // Finish writing stream in the background
        writer.write_background()?;

        // Wait on receiver to get the response
        rx.recv().map_err(|_| ShellError::NushellFailed {
            msg: "Failed to get response to engine call because the channel was closed".into(),
        })
    }

    /// Get the full shell configuration from the engine. As this is quite a large object, it is
    /// provided on request only.
    ///
    /// # Example
    ///
    /// Format a value in the user's preferred way:
    ///
    /// ```rust,no_run
    /// # use nu_protocol::{Value, ShellError};
    /// # use nu_plugin::EngineInterface;
    /// # fn example(engine: &EngineInterface, value: &Value) -> Result<(), ShellError> {
    /// let config = engine.get_config()?;
    /// eprintln!("{}", value.to_expanded_string(", ", &config));
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_config(&self) -> Result<Box<Config>, ShellError> {
        match self.engine_call(EngineCall::GetConfig)? {
            EngineCallResponse::Config(config) => Ok(config),
            EngineCallResponse::Error(err) => Err(err),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response for EngineCall::GetConfig".into(),
            }),
        }
    }

    /// Do an engine call returning an `Option<Value>` as either `PipelineData::Empty` or
    /// `PipelineData::Value`
    fn engine_call_option_value(
        &self,
        engine_call: EngineCall<PipelineData>,
    ) -> Result<Option<Value>, ShellError> {
        let name = engine_call.name();
        match self.engine_call(engine_call)? {
            EngineCallResponse::PipelineData(PipelineData::Empty) => Ok(None),
            EngineCallResponse::PipelineData(PipelineData::Value(value, _)) => Ok(Some(value)),
            EngineCallResponse::Error(err) => Err(err),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: format!("Received unexpected response for EngineCall::{name}"),
            }),
        }
    }

    /// Get the plugin-specific configuration from the engine. This lives in
    /// `$env.config.plugins.NAME` for a plugin named `NAME`. If the config is set to a closure,
    /// it is automatically evaluated each time.
    ///
    /// # Example
    ///
    /// Print this plugin's config:
    ///
    /// ```rust,no_run
    /// # use nu_protocol::{Value, ShellError};
    /// # use nu_plugin::EngineInterface;
    /// # fn example(engine: &EngineInterface, value: &Value) -> Result<(), ShellError> {
    /// let config = engine.get_plugin_config()?;
    /// eprintln!("{:?}", config);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_plugin_config(&self) -> Result<Option<Value>, ShellError> {
        self.engine_call_option_value(EngineCall::GetPluginConfig)
    }

    /// Get an environment variable from the engine.
    ///
    /// Returns `Some(value)` if present, and `None` if not found.
    ///
    /// # Example
    ///
    /// Get `$env.PATH`:
    ///
    /// ```rust,no_run
    /// # use nu_protocol::{Value, ShellError};
    /// # use nu_plugin::EngineInterface;
    /// # fn example(engine: &EngineInterface) -> Result<Option<Value>, ShellError> {
    /// engine.get_env_var("PATH") // => Ok(Some(Value::List([...])))
    /// # }
    /// ```
    pub fn get_env_var(&self, name: impl Into<String>) -> Result<Option<Value>, ShellError> {
        self.engine_call_option_value(EngineCall::GetEnvVar(name.into()))
    }

    /// Get the current working directory from the engine. The result is always an absolute path.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use nu_protocol::{Value, ShellError};
    /// # use nu_plugin::EngineInterface;
    /// # fn example(engine: &EngineInterface) -> Result<String, ShellError> {
    /// engine.get_current_dir() // => "/home/user"
    /// # }
    /// ```
    pub fn get_current_dir(&self) -> Result<String, ShellError> {
        match self.engine_call(EngineCall::GetCurrentDir)? {
            // Always a string, and the span doesn't matter.
            EngineCallResponse::PipelineData(PipelineData::Value(Value::String { val, .. }, _)) => {
                Ok(val)
            }
            EngineCallResponse::Error(err) => Err(err),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response for EngineCall::GetCurrentDir".into(),
            }),
        }
    }

    /// Get all environment variables from the engine.
    ///
    /// Since this is quite a large map that has to be sent, prefer to use [`.get_env_var()`] if
    /// the variables needed are known ahead of time and there are only a small number needed.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use nu_protocol::{Value, ShellError};
    /// # use nu_plugin::EngineInterface;
    /// # use std::collections::HashMap;
    /// # fn example(engine: &EngineInterface) -> Result<HashMap<String, Value>, ShellError> {
    /// engine.get_env_vars() // => Ok({"PATH": Value::List([...]), ...})
    /// # }
    /// ```
    pub fn get_env_vars(&self) -> Result<HashMap<String, Value>, ShellError> {
        match self.engine_call(EngineCall::GetEnvVars)? {
            EngineCallResponse::ValueMap(map) => Ok(map),
            EngineCallResponse::Error(err) => Err(err),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response type for EngineCall::GetEnvVars".into(),
            }),
        }
    }

    /// Ask the engine to evaluate a closure. Input to the closure is passed as a stream, and the
    /// output is available as a stream.
    ///
    /// Set `redirect_stdout` to `true` to capture the standard output stream of an external
    /// command, if the closure results in an external command.
    ///
    /// Set `redirect_stderr` to `true` to capture the standard error stream of an external command,
    /// if the closure results in an external command.
    ///
    /// # Example
    ///
    /// Invoked as:
    ///
    /// ```nushell
    /// my_command { seq 1 $in | each { |n| $"Hello, ($n)" } }
    /// ```
    ///
    /// ```rust,no_run
    /// # use nu_protocol::{Value, ShellError, PipelineData};
    /// # use nu_plugin::{EngineInterface, EvaluatedCall};
    /// # fn example(engine: &EngineInterface, call: &EvaluatedCall) -> Result<(), ShellError> {
    /// let closure = call.req(0)?;
    /// let input = PipelineData::Value(Value::int(4, call.head), None);
    /// let output = engine.eval_closure_with_stream(
    ///     &closure,
    ///     vec![],
    ///     input,
    ///     true,
    ///     false,
    /// )?;
    /// for value in output {
    ///     eprintln!("Closure says: {}", value.as_str()?);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Output:
    ///
    /// ```text
    /// Closure says: Hello, 1
    /// Closure says: Hello, 2
    /// Closure says: Hello, 3
    /// Closure says: Hello, 4
    /// ```
    pub fn eval_closure_with_stream(
        &self,
        closure: &Spanned<Closure>,
        mut positional: Vec<Value>,
        input: PipelineData,
        redirect_stdout: bool,
        redirect_stderr: bool,
    ) -> Result<PipelineData, ShellError> {
        // Ensure closure args have custom values serialized
        positional
            .iter_mut()
            .try_for_each(PluginCustomValue::serialize_custom_values_in)?;

        let call = EngineCall::EvalClosure {
            closure: closure.clone(),
            positional,
            input,
            redirect_stdout,
            redirect_stderr,
        };

        match self.engine_call(call)? {
            EngineCallResponse::Error(error) => Err(error),
            EngineCallResponse::PipelineData(data) => Ok(data),
            _ => Err(ShellError::PluginFailedToDecode {
                msg: "Received unexpected response type for EngineCall::EvalClosure".into(),
            }),
        }
    }

    /// Ask the engine to evaluate a closure. Input is optionally passed as a [`Value`], and output
    /// of the closure is collected to a [`Value`] even if it is a stream.
    ///
    /// If the closure results in an external command, the return value will be a collected string
    /// or binary value of the standard output stream of that command, similar to calling
    /// [`eval_closure_with_stream()`](Self::eval_closure_with_stream) with `redirect_stdout` =
    /// `true` and `redirect_stderr` = `false`.
    ///
    /// Use [`eval_closure_with_stream()`](Self::eval_closure_with_stream) if more control over the
    /// input and output is desired.
    ///
    /// # Example
    ///
    /// Invoked as:
    ///
    /// ```nushell
    /// my_command { |number| $number + 1}
    /// ```
    ///
    /// ```rust,no_run
    /// # use nu_protocol::{Value, ShellError};
    /// # use nu_plugin::{EngineInterface, EvaluatedCall};
    /// # fn example(engine: &EngineInterface, call: &EvaluatedCall) -> Result<(), ShellError> {
    /// let closure = call.req(0)?;
    /// for n in 0..4 {
    ///     let result = engine.eval_closure(&closure, vec![Value::int(n, call.head)], None)?;
    ///     eprintln!("{} => {}", n, result.as_int()?);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Output:
    ///
    /// ```text
    /// 0 => 1
    /// 1 => 2
    /// 2 => 3
    /// 3 => 4
    /// ```
    pub fn eval_closure(
        &self,
        closure: &Spanned<Closure>,
        positional: Vec<Value>,
        input: Option<Value>,
    ) -> Result<Value, ShellError> {
        let input = input.map_or_else(|| PipelineData::Empty, |v| PipelineData::Value(v, None));
        let output = self.eval_closure_with_stream(closure, positional, input, true, false)?;
        // Unwrap an error value
        match output.into_value(closure.span) {
            Value::Error { error, .. } => Err(*error),
            value => Ok(value),
        }
    }

    /// Tell the engine whether to disable garbage collection for this plugin.
    ///
    /// The garbage collector is enabled by default, but plugins can turn it off (ideally
    /// temporarily) as necessary to implement functionality that requires the plugin to stay
    /// running for longer than the engine can automatically determine.
    ///
    /// The user can still stop the plugin if they want to with the `plugin stop` command.
    pub fn set_gc_disabled(&self, disabled: bool) -> Result<(), ShellError> {
        self.write(PluginOutput::Option(PluginOption::GcDisabled(disabled)))?;
        self.flush()
    }

    /// Write a call response of [`Ordering`], for `partial_cmp`.
    pub(crate) fn write_ordering(
        &self,
        ordering: Option<impl Into<Ordering>>,
    ) -> Result<(), ShellError> {
        let response = PluginCallResponse::Ordering(ordering.map(|o| o.into()));
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
