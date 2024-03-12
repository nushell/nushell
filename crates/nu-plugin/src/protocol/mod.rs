mod evaluated_call;
mod plugin_custom_value;
mod protocol_info;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_util;

use std::collections::HashMap;

pub use evaluated_call::EvaluatedCall;
use nu_protocol::{
    ast::Operator, engine::Closure, Config, PipelineData, PluginSignature, RawStream, ShellError,
    Span, Spanned, Value,
};
pub use plugin_custom_value::PluginCustomValue;
pub use protocol_info::ProtocolInfo;
#[cfg(test)]
pub use protocol_info::{Feature, Protocol};
use serde::{Deserialize, Serialize};

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
#[derive(Serialize, Deserialize, Debug, Clone)]
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
    /// Stream control or data message. Untagged to keep them as small as possible.
    ///
    /// For example, `Stream(Ack(0))` is encoded as `{"Ack": 0}`
    #[serde(untagged)]
    Stream(StreamMessage),
}

impl TryFrom<PluginInput> for StreamMessage {
    type Error = PluginInput;

    fn try_from(msg: PluginInput) -> Result<StreamMessage, PluginInput> {
        match msg {
            PluginInput::Stream(stream_msg) => Ok(stream_msg),
            _ => Err(msg),
        }
    }
}

impl From<StreamMessage> for PluginInput {
    fn from(stream_msg: StreamMessage) -> PluginInput {
        PluginInput::Stream(stream_msg)
    }
}

/// A single item of stream data for a stream.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StreamData {
    List(Value),
    Raw(Result<Vec<u8>, ShellError>),
}

impl From<Value> for StreamData {
    fn from(value: Value) -> Self {
        StreamData::List(value)
    }
}

impl From<Result<Vec<u8>, ShellError>> for StreamData {
    fn from(value: Result<Vec<u8>, ShellError>) -> Self {
        StreamData::Raw(value)
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

impl TryFrom<StreamData> for Result<Vec<u8>, ShellError> {
    type Error = ShellError;

    fn try_from(data: StreamData) -> Result<Result<Vec<u8>, ShellError>, ShellError> {
        match data {
            StreamData::Raw(value) => Ok(value),
            StreamData::List(_) => Err(ShellError::PluginFailedToDecode {
                msg: "expected raw stream data, found list data".into(),
            }),
        }
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

/// An error message with debugging information that can be passed to Nushell from the plugin
///
/// The `LabeledError` struct is a structured error message that can be returned from
/// a [Plugin](crate::Plugin)'s [`run`](crate::Plugin::run()) method. It contains
/// the error message along with optional [Span] data to support highlighting in the
/// shell.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct LabeledError {
    /// The name of the error
    pub label: String,
    /// A detailed error description
    pub msg: String,
    /// The [Span] in which the error occurred
    pub span: Option<Span>,
}

impl From<LabeledError> for ShellError {
    fn from(error: LabeledError) -> Self {
        if error.span.is_some() {
            ShellError::GenericError {
                error: error.label,
                msg: error.msg,
                span: error.span,
                help: None,
                inner: vec![],
            }
        } else {
            ShellError::GenericError {
                error: error.label,
                msg: "".into(),
                span: None,
                help: (!error.msg.is_empty()).then_some(error.msg),
                inner: vec![],
            }
        }
    }
}

impl From<ShellError> for LabeledError {
    fn from(error: ShellError) -> Self {
        use miette::Diagnostic;
        // This is not perfect - we can only take the first labeled span as that's all we have
        // space for.
        if let Some(labeled_span) = error.labels().and_then(|mut iter| iter.nth(0)) {
            let offset = labeled_span.offset();
            let span = Span::new(offset, offset + labeled_span.len());
            LabeledError {
                label: error.to_string(),
                msg: labeled_span
                    .label()
                    .map(|label| label.to_owned())
                    .unwrap_or_else(|| "".into()),
                span: Some(span),
            }
        } else {
            LabeledError {
                label: error.to_string(),
                msg: error
                    .help()
                    .map(|help| help.to_string())
                    .unwrap_or_else(|| "".into()),
                span: None,
            }
        }
    }
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

/// Options that can be changed to affect how the engine treats the plugin
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PluginOption {
    /// Send `GcDisabled(true)` to stop the plugin from being automatically garbage collected, or
    /// `GcDisabled(false)` to enable it again.
    ///
    /// See [`EngineInterface::set_gc_disabled`] for more information.
    GcDisabled(bool),
}

/// This is just a serializable version of [std::cmp::Ordering], and can be converted 1:1
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
    /// Stream control or data message. Untagged to keep them as small as possible.
    ///
    /// For example, `Stream(Ack(0))` is encoded as `{"Ack": 0}`
    #[serde(untagged)]
    Stream(StreamMessage),
}

impl TryFrom<PluginOutput> for StreamMessage {
    type Error = PluginOutput;

    fn try_from(msg: PluginOutput) -> Result<StreamMessage, PluginOutput> {
        match msg {
            PluginOutput::Stream(stream_msg) => Ok(stream_msg),
            _ => Err(msg),
        }
    }
}

impl From<StreamMessage> for PluginOutput {
    fn from(stream_msg: StreamMessage) -> PluginOutput {
        PluginOutput::Stream(stream_msg)
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
            EngineCall::EvalClosure { .. } => "EvalClosure",
        }
    }
}

/// The response to an [EngineCall]. The type parameter determines the output type for pipeline
/// data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EngineCallResponse<D> {
    Error(ShellError),
    PipelineData(D),
    Config(Box<Config>),
    ValueMap(HashMap<String, Value>),
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
