mod evaluated_call;
mod plugin_custom_value;
mod protocol_info;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_util;

pub use evaluated_call::EvaluatedCall;
use nu_protocol::{PluginSignature, RawStream, ShellError, Span, Spanned, Value};
pub use plugin_custom_value::PluginCustomValue;
pub(crate) use protocol_info::ProtocolInfo;
use serde::{Deserialize, Serialize};

#[cfg(test)]
pub(crate) use protocol_info::Protocol;

/// A sequential identifier for a stream
pub type StreamId = usize;

/// A sequential identifier for a [`PluginCall`]
pub type PluginCallId = usize;

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
    /// Plugin configuration, if available
    pub config: Option<Value>,
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

/// Information received from the plugin
///
/// Note: exported for internal use, not public.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[doc(hidden)]
pub enum PluginOutput {
    /// This must be the first message. Indicates supported protocol
    Hello(ProtocolInfo),
    /// A response to a [`PluginCall`]. The ID should be the same sent with the plugin call this
    /// is a response to
    CallResponse(PluginCallId, PluginCallResponse<PipelineDataHeader>),
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
