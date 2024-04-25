mod evaluated_call;
mod plugin_custom_value;
mod protocol_info;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_util;

use nu_protocol::{
    ast::Operator, engine::Closure, Config, LabeledError, PipelineData, PluginSignature, RawStream,
    ShellError, Span, Spanned, Value,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use evaluated_call::EvaluatedCall;
pub use plugin_custom_value::PluginCustomValue;
#[allow(unused_imports)] // may be unused by compile flags
pub use protocol_info::{Feature, Protocol, ProtocolInfo};

/// A sequential identifier for a stream
pub type StreamId = usize;

/// A sequential identifier for a [`PluginCall`]
pub type PluginCallId = usize;

/// A sequential identifier for an [`EngineCall`]
pub type EngineCallId = usize;

/// Information about a plugin command invocation. This includes an [`EvaluatedCall`] as a
/// serializable representation of [`nu_protocol::ast::Call`]. The type parameter determines
/// the input type.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CallInfo<D> {
    /// The name of the command to be run
    pub name: String,
    /// Information about the invocation, including arguments
    pub call: EvaluatedCall,
    /// Pipeline input. This is usually [`nu_protocol::PipelineData`] or [`PipelineDataHeader`]
    pub input: D,
}

impl<D> CallInfo<D> {
    /// Convert the type of `input` from `D` to `T`.
    pub(crate) fn map_data<T>(
        self,
        f: impl FnOnce(D) -> Result<T, ShellError>,
    ) -> Result<CallInfo<T>, ShellError> {
        Ok(CallInfo {
            name: self.name,
            call: self.call,
            input: f(self.input)?,
        })
    }
}

/// The initial (and perhaps only) part of any [`nu_protocol::PipelineData`] sent over the wire.
///
/// This may contain a single value, or may initiate a stream with a [`StreamId`].
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum PipelineDataHeader {
    /// No input
    Empty,
    /// A single value
    Value(Value),
    /// Initiate [`nu_protocol::PipelineData::ListStream`].
    ///
    /// Items are sent via [`StreamData`]
    ListStream(ListStreamInfo),
    /// Initiate [`nu_protocol::PipelineData::ExternalStream`].
    ///
    /// Items are sent via [`StreamData`]
    ExternalStream(ExternalStreamInfo),
}

impl PipelineDataHeader {
    /// Return a list of stream IDs embedded in the header
    pub(crate) fn stream_ids(&self) -> Vec<StreamId> {
        match self {
            PipelineDataHeader::Empty => vec![],
            PipelineDataHeader::Value(_) => vec![],
            PipelineDataHeader::ListStream(info) => vec![info.id],
            PipelineDataHeader::ExternalStream(info) => {
                let mut out = vec![];
                if let Some(stdout) = &info.stdout {
                    out.push(stdout.id);
                }
                if let Some(stderr) = &info.stderr {
                    out.push(stderr.id);
                }
                if let Some(exit_code) = &info.exit_code {
                    out.push(exit_code.id);
                }
                out
            }
        }
    }
}

/// Additional information about list (value) streams
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ListStreamInfo {
    pub id: StreamId,
}

/// Additional information about external streams
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ExternalStreamInfo {
    pub span: Span,
    pub stdout: Option<RawStreamInfo>,
    pub stderr: Option<RawStreamInfo>,
    pub exit_code: Option<ListStreamInfo>,
    pub trim_end_newline: bool,
}

/// Additional information about raw (byte) streams
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RawStreamInfo {
    pub id: StreamId,
    pub is_binary: bool,
    pub known_size: Option<u64>,
}

impl RawStreamInfo {
    pub(crate) fn new(id: StreamId, stream: &RawStream) -> Self {
        RawStreamInfo {
            id,
            is_binary: stream.is_binary,
            known_size: stream.known_size,
        }
    }
}

/// Calls that a plugin can execute. The type parameter determines the input type.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PluginCall<D> {
    Signature,
    Run(CallInfo<D>),
    CustomValueOp(Spanned<PluginCustomValue>, CustomValueOp),
}

impl<D> PluginCall<D> {
    /// Convert the data type from `D` to `T`. The function will not be called if the variant does
    /// not contain data.
    pub(crate) fn map_data<T>(
        self,
        f: impl FnOnce(D) -> Result<T, ShellError>,
    ) -> Result<PluginCall<T>, ShellError> {
        Ok(match self {
            PluginCall::Signature => PluginCall::Signature,
            PluginCall::Run(call) => PluginCall::Run(call.map_data(f)?),
            PluginCall::CustomValueOp(custom_value, op) => {
                PluginCall::CustomValueOp(custom_value, op)
            }
        })
    }

    /// The span associated with the call.
    pub fn span(&self) -> Option<Span> {
        match self {
            PluginCall::Signature => None,
            PluginCall::Run(CallInfo { call, .. }) => Some(call.head),
            PluginCall::CustomValueOp(val, _) => Some(val.span),
        }
    }
}

/// Operations supported for custom values.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CustomValueOp {
    /// [`to_base_value()`](nu_protocol::CustomValue::to_base_value)
    ToBaseValue,
    /// [`follow_path_int()`](nu_protocol::CustomValue::follow_path_int)
    FollowPathInt(Spanned<usize>),
    /// [`follow_path_string()`](nu_protocol::CustomValue::follow_path_string)
    FollowPathString(Spanned<String>),
    /// [`partial_cmp()`](nu_protocol::CustomValue::partial_cmp)
    PartialCmp(Value),
    /// [`operation()`](nu_protocol::CustomValue::operation)
    Operation(Spanned<Operator>, Value),
    /// Notify that the custom value has been dropped, if
    /// [`notify_plugin_on_drop()`](nu_protocol::CustomValue::notify_plugin_on_drop) is true
    Dropped,
}

impl CustomValueOp {
    /// Get the name of the op, for error messages.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            CustomValueOp::ToBaseValue => "to_base_value",
            CustomValueOp::FollowPathInt(_) => "follow_path_int",
            CustomValueOp::FollowPathString(_) => "follow_path_string",
            CustomValueOp::PartialCmp(_) => "partial_cmp",
            CustomValueOp::Operation(_, _) => "operation",
            CustomValueOp::Dropped => "dropped",
        }
    }
}

/// Any data sent to the plugin
///
/// Note: exported for internal use, not public.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[doc(hidden)]
pub enum PluginInput {
    /// This must be the first message. Indicates supported protocol
    Hello(ProtocolInfo),
    /// Execute a [`PluginCall`], such as `Run` or `Signature`. The ID should not have been used
    /// before.
    Call(PluginCallId, PluginCall<PipelineDataHeader>),
    /// Don't expect any more plugin calls. Exit after all currently executing plugin calls are
    /// finished.
    Goodbye,
    /// Response to an [`EngineCall`]. The ID should be the same one sent with the engine call this
    /// is responding to
    EngineCallResponse(EngineCallId, EngineCallResponse<PipelineDataHeader>),
    /// See [`StreamMessage::Data`].
    Data(StreamId, StreamData),
    /// See [`StreamMessage::End`].
    End(StreamId),
    /// See [`StreamMessage::Drop`].
    Drop(StreamId),
    /// See [`StreamMessage::Ack`].
    Ack(StreamId),
}

impl TryFrom<PluginInput> for StreamMessage {
    type Error = PluginInput;

    fn try_from(msg: PluginInput) -> Result<StreamMessage, PluginInput> {
        match msg {
            PluginInput::Data(id, data) => Ok(StreamMessage::Data(id, data)),
            PluginInput::End(id) => Ok(StreamMessage::End(id)),
            PluginInput::Drop(id) => Ok(StreamMessage::Drop(id)),
            PluginInput::Ack(id) => Ok(StreamMessage::Ack(id)),
            _ => Err(msg),
        }
    }
}

impl From<StreamMessage> for PluginInput {
    fn from(stream_msg: StreamMessage) -> PluginInput {
        match stream_msg {
            StreamMessage::Data(id, data) => PluginInput::Data(id, data),
            StreamMessage::End(id) => PluginInput::End(id),
            StreamMessage::Drop(id) => PluginInput::Drop(id),
            StreamMessage::Ack(id) => PluginInput::Ack(id),
        }
    }
}

/// A single item of stream data for a stream.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StreamData {
    List(Value),
    Raw(Result<Vec<u8>, LabeledError>),
}

impl From<Value> for StreamData {
    fn from(value: Value) -> Self {
        StreamData::List(value)
    }
}

impl From<Result<Vec<u8>, LabeledError>> for StreamData {
    fn from(value: Result<Vec<u8>, LabeledError>) -> Self {
        StreamData::Raw(value)
    }
}

impl From<Result<Vec<u8>, ShellError>> for StreamData {
    fn from(value: Result<Vec<u8>, ShellError>) -> Self {
        value.map_err(LabeledError::from).into()
    }
}

impl TryFrom<StreamData> for Value {
    type Error = ShellError;

    fn try_from(data: StreamData) -> Result<Value, ShellError> {
        match data {
            StreamData::List(value) => Ok(value),
            StreamData::Raw(_) => Err(ShellError::PluginFailedToDecode {
                msg: "expected list stream data, found raw data".into(),
            }),
        }
    }
}

impl TryFrom<StreamData> for Result<Vec<u8>, LabeledError> {
    type Error = ShellError;

    fn try_from(data: StreamData) -> Result<Result<Vec<u8>, LabeledError>, ShellError> {
        match data {
            StreamData::Raw(value) => Ok(value),
            StreamData::List(_) => Err(ShellError::PluginFailedToDecode {
                msg: "expected raw stream data, found list data".into(),
            }),
        }
    }
}

impl TryFrom<StreamData> for Result<Vec<u8>, ShellError> {
    type Error = ShellError;

    fn try_from(value: StreamData) -> Result<Result<Vec<u8>, ShellError>, ShellError> {
        Result::<Vec<u8>, LabeledError>::try_from(value).map(|res| res.map_err(ShellError::from))
    }
}

/// A stream control or data message.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StreamMessage {
    /// Append data to the stream. Sent by the stream producer.
    Data(StreamId, StreamData),
    /// End of stream. Sent by the stream producer.
    End(StreamId),
    /// Notify that the read end of the stream has closed, and further messages should not be
    /// sent. Sent by the stream consumer.
    Drop(StreamId),
    /// Acknowledge that a message has been consumed. This is used to implement flow control by
    /// the stream producer. Sent by the stream consumer.
    Ack(StreamId),
}

/// Response to a [`PluginCall`]. The type parameter determines the output type for pipeline data.
///
/// Note: exported for internal use, not public.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[doc(hidden)]
pub enum PluginCallResponse<D> {
    Error(LabeledError),
    Signature(Vec<PluginSignature>),
    Ordering(Option<Ordering>),
    PipelineData(D),
}

impl<D> PluginCallResponse<D> {
    /// Convert the data type from `D` to `T`. The function will not be called if the variant does
    /// not contain data.
    pub(crate) fn map_data<T>(
        self,
        f: impl FnOnce(D) -> Result<T, ShellError>,
    ) -> Result<PluginCallResponse<T>, ShellError> {
        Ok(match self {
            PluginCallResponse::Error(err) => PluginCallResponse::Error(err),
            PluginCallResponse::Signature(sigs) => PluginCallResponse::Signature(sigs),
            PluginCallResponse::Ordering(ordering) => PluginCallResponse::Ordering(ordering),
            PluginCallResponse::PipelineData(input) => PluginCallResponse::PipelineData(f(input)?),
        })
    }
}

impl PluginCallResponse<PipelineDataHeader> {
    /// Construct a plugin call response with a single value
    pub fn value(value: Value) -> PluginCallResponse<PipelineDataHeader> {
        if value.is_nothing() {
            PluginCallResponse::PipelineData(PipelineDataHeader::Empty)
        } else {
            PluginCallResponse::PipelineData(PipelineDataHeader::Value(value))
        }
    }
}

impl PluginCallResponse<PipelineData> {
    /// Does this response have a stream?
    pub(crate) fn has_stream(&self) -> bool {
        match self {
            PluginCallResponse::PipelineData(data) => match data {
                PipelineData::Empty => false,
                PipelineData::Value(..) => false,
                PipelineData::ListStream(..) => true,
                PipelineData::ExternalStream { .. } => true,
            },
            _ => false,
        }
    }
}

/// Options that can be changed to affect how the engine treats the plugin
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PluginOption {
    /// Send `GcDisabled(true)` to stop the plugin from being automatically garbage collected, or
    /// `GcDisabled(false)` to enable it again.
    ///
    /// See [`EngineInterface::set_gc_disabled`] for more information.
    GcDisabled(bool),
}

/// This is just a serializable version of [`std::cmp::Ordering`], and can be converted 1:1
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Ordering {
    Less,
    Equal,
    Greater,
}

impl From<std::cmp::Ordering> for Ordering {
    fn from(value: std::cmp::Ordering) -> Self {
        match value {
            std::cmp::Ordering::Less => Ordering::Less,
            std::cmp::Ordering::Equal => Ordering::Equal,
            std::cmp::Ordering::Greater => Ordering::Greater,
        }
    }
}

impl From<Ordering> for std::cmp::Ordering {
    fn from(value: Ordering) -> Self {
        match value {
            Ordering::Less => std::cmp::Ordering::Less,
            Ordering::Equal => std::cmp::Ordering::Equal,
            Ordering::Greater => std::cmp::Ordering::Greater,
        }
    }
}

/// Information received from the plugin
///
/// Note: exported for internal use, not public.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[doc(hidden)]
pub enum PluginOutput {
    /// This must be the first message. Indicates supported protocol
    Hello(ProtocolInfo),
    /// Set option. No response expected
    Option(PluginOption),
    /// A response to a [`PluginCall`]. The ID should be the same sent with the plugin call this
    /// is a response to
    CallResponse(PluginCallId, PluginCallResponse<PipelineDataHeader>),
    /// Execute an [`EngineCall`]. Engine calls must be executed within the `context` of a plugin
    /// call, and the `id` should not have been used before
    EngineCall {
        /// The plugin call (by ID) to execute in the context of
        context: PluginCallId,
        /// A new identifier for this engine call. The response will reference this ID
        id: EngineCallId,
        call: EngineCall<PipelineDataHeader>,
    },
    /// See [`StreamMessage::Data`].
    Data(StreamId, StreamData),
    /// See [`StreamMessage::End`].
    End(StreamId),
    /// See [`StreamMessage::Drop`].
    Drop(StreamId),
    /// See [`StreamMessage::Ack`].
    Ack(StreamId),
}

impl TryFrom<PluginOutput> for StreamMessage {
    type Error = PluginOutput;

    fn try_from(msg: PluginOutput) -> Result<StreamMessage, PluginOutput> {
        match msg {
            PluginOutput::Data(id, data) => Ok(StreamMessage::Data(id, data)),
            PluginOutput::End(id) => Ok(StreamMessage::End(id)),
            PluginOutput::Drop(id) => Ok(StreamMessage::Drop(id)),
            PluginOutput::Ack(id) => Ok(StreamMessage::Ack(id)),
            _ => Err(msg),
        }
    }
}

impl From<StreamMessage> for PluginOutput {
    fn from(stream_msg: StreamMessage) -> PluginOutput {
        match stream_msg {
            StreamMessage::Data(id, data) => PluginOutput::Data(id, data),
            StreamMessage::End(id) => PluginOutput::End(id),
            StreamMessage::Drop(id) => PluginOutput::Drop(id),
            StreamMessage::Ack(id) => PluginOutput::Ack(id),
        }
    }
}

/// A remote call back to the engine during the plugin's execution.
///
/// The type parameter determines the input type, for calls that take pipeline data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EngineCall<D> {
    /// Get the full engine configuration
    GetConfig,
    /// Get the plugin-specific configuration (`$env.config.plugins.NAME`)
    GetPluginConfig,
    /// Get an environment variable
    GetEnvVar(String),
    /// Get all environment variables
    GetEnvVars,
    /// Get current working directory
    GetCurrentDir,
    /// Set an environment variable in the caller's scope
    AddEnvVar(String, Value),
    /// Get help for the current command
    GetHelp,
    /// Move the plugin into the foreground for terminal interaction
    EnterForeground,
    /// Move the plugin out of the foreground once terminal interaction has finished
    LeaveForeground,
    /// Get the contents of a span. Response is a binary which may not parse to UTF-8
    GetSpanContents(Span),
    /// Evaluate a closure with stream input/output
    EvalClosure {
        /// The closure to call.
        ///
        /// This may come from a [`Value::Closure`] passed in as an argument to the plugin.
        closure: Spanned<Closure>,
        /// Positional arguments to add to the closure call
        positional: Vec<Value>,
        /// Input to the closure
        input: D,
        /// Whether to redirect stdout from external commands
        redirect_stdout: bool,
        /// Whether to redirect stderr from external commands
        redirect_stderr: bool,
    },
}

impl<D> EngineCall<D> {
    /// Get the name of the engine call so it can be embedded in things like error messages
    pub fn name(&self) -> &'static str {
        match self {
            EngineCall::GetConfig => "GetConfig",
            EngineCall::GetPluginConfig => "GetPluginConfig",
            EngineCall::GetEnvVar(_) => "GetEnv",
            EngineCall::GetEnvVars => "GetEnvs",
            EngineCall::GetCurrentDir => "GetCurrentDir",
            EngineCall::AddEnvVar(..) => "AddEnvVar",
            EngineCall::GetHelp => "GetHelp",
            EngineCall::EnterForeground => "EnterForeground",
            EngineCall::LeaveForeground => "LeaveForeground",
            EngineCall::GetSpanContents(_) => "GetSpanContents",
            EngineCall::EvalClosure { .. } => "EvalClosure",
        }
    }

    /// Convert the data type from `D` to `T`. The function will not be called if the variant does
    /// not contain data.
    pub(crate) fn map_data<T>(
        self,
        f: impl FnOnce(D) -> Result<T, ShellError>,
    ) -> Result<EngineCall<T>, ShellError> {
        Ok(match self {
            EngineCall::GetConfig => EngineCall::GetConfig,
            EngineCall::GetPluginConfig => EngineCall::GetPluginConfig,
            EngineCall::GetEnvVar(name) => EngineCall::GetEnvVar(name),
            EngineCall::GetEnvVars => EngineCall::GetEnvVars,
            EngineCall::GetCurrentDir => EngineCall::GetCurrentDir,
            EngineCall::AddEnvVar(name, value) => EngineCall::AddEnvVar(name, value),
            EngineCall::GetHelp => EngineCall::GetHelp,
            EngineCall::EnterForeground => EngineCall::EnterForeground,
            EngineCall::LeaveForeground => EngineCall::LeaveForeground,
            EngineCall::GetSpanContents(span) => EngineCall::GetSpanContents(span),
            EngineCall::EvalClosure {
                closure,
                positional,
                input,
                redirect_stdout,
                redirect_stderr,
            } => EngineCall::EvalClosure {
                closure,
                positional,
                input: f(input)?,
                redirect_stdout,
                redirect_stderr,
            },
        })
    }
}

/// The response to an [`EngineCall`]. The type parameter determines the output type for pipeline
/// data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EngineCallResponse<D> {
    Error(ShellError),
    PipelineData(D),
    Config(Box<Config>),
    ValueMap(HashMap<String, Value>),
}

impl<D> EngineCallResponse<D> {
    /// Convert the data type from `D` to `T`. The function will not be called if the variant does
    /// not contain data.
    pub(crate) fn map_data<T>(
        self,
        f: impl FnOnce(D) -> Result<T, ShellError>,
    ) -> Result<EngineCallResponse<T>, ShellError> {
        Ok(match self {
            EngineCallResponse::Error(err) => EngineCallResponse::Error(err),
            EngineCallResponse::PipelineData(data) => EngineCallResponse::PipelineData(f(data)?),
            EngineCallResponse::Config(config) => EngineCallResponse::Config(config),
            EngineCallResponse::ValueMap(map) => EngineCallResponse::ValueMap(map),
        })
    }
}

impl EngineCallResponse<PipelineData> {
    /// Build an [`EngineCallResponse::PipelineData`] from a [`Value`]
    pub(crate) fn value(value: Value) -> EngineCallResponse<PipelineData> {
        EngineCallResponse::PipelineData(PipelineData::Value(value, None))
    }

    /// An [`EngineCallResponse::PipelineData`] with [`PipelineData::Empty`]
    pub(crate) const fn empty() -> EngineCallResponse<PipelineData> {
        EngineCallResponse::PipelineData(PipelineData::Empty)
    }
}
