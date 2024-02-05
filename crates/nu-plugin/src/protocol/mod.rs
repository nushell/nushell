mod evaluated_call;
mod plugin_custom_value;
mod plugin_data;

pub use evaluated_call::EvaluatedCall;
use nu_protocol::{PluginSignature, RawStream, ShellError, Span, Value};
pub use plugin_custom_value::PluginCustomValue;
pub use plugin_data::PluginData;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CallInfo {
    pub name: String,
    pub call: EvaluatedCall,
    pub input: CallInput,
    pub config: Option<Value>,
}

/// Pipeline input to a plugin call
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum CallInput {
    /// No input
    Empty,
    /// A single value
    Value(Value),
    /// Deserialized to [PluginCustomValue]
    Data(PluginData),
    /// Initiate [nu_protocol::PipelineData::ListStream]
    ///
    /// Items are sent via [StreamData]
    ListStream,
    /// Initiate [nu_protocol::PipelineData::ExternalStream]
    ///
    /// Items are sent via [StreamData]
    ExternalStream(ExternalStreamInfo),
}

/// Additional information about external streams
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ExternalStreamInfo {
    pub span: Span,
    pub stdout: Option<RawStreamInfo>,
    pub stderr: Option<RawStreamInfo>,
    pub has_exit_code: bool,
    pub trim_end_newline: bool,
}

/// Additional information about raw streams
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RawStreamInfo {
    pub is_binary: bool,
    pub known_size: Option<u64>,
}

impl From<&RawStream> for RawStreamInfo {
    fn from(stream: &RawStream) -> Self {
        RawStreamInfo {
            is_binary: stream.is_binary,
            known_size: stream.known_size,
        }
    }
}

/// Initial message sent to the plugin
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PluginCall {
    Signature,
    Run(CallInfo),
    CollapseCustomValue(PluginData),
}

/// Any data sent to the plugin
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PluginInput {
    Call(PluginCall),
    StreamData(StreamData),
}

/// A single item of stream data for a stream.
///
/// A `None` value ends the stream. An `Error` ends all streams, and the error should be propagated.
///
/// Note: exported for internal use, not public.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[doc(hidden)]
pub enum StreamData {
    List(Option<Value>),
    ExternalStdout(Option<Result<Vec<u8>, ShellError>>),
    ExternalStderr(Option<Result<Vec<u8>, ShellError>>),
    ExternalExitCode(Option<Value>),
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
        match error.span {
            Some(span) => ShellError::GenericError {
                error: error.label,
                msg: error.msg,
                span: Some(span),
                help: None,
                inner: vec![],
            },
            None => ShellError::GenericError {
                error: error.label,
                msg: "".into(),
                span: None,
                help: Some(error.msg),
                inner: vec![],
            },
        }
    }
}

impl From<ShellError> for LabeledError {
    fn from(error: ShellError) -> Self {
        match error {
            ShellError::GenericError {
                error: label,
                msg,
                span,
                ..
            } => LabeledError { label, msg, span },
            ShellError::CantConvert {
                to_type: expected,
                from_type: input,
                span,
                help: _help,
            } => LabeledError {
                label: format!("Can't convert to {expected}"),
                msg: format!("can't convert from {input} to {expected}"),
                span: Some(span),
            },
            ShellError::DidYouMean { suggestion, span } => LabeledError {
                label: "Name not found".into(),
                msg: format!("did you mean '{suggestion}'?"),
                span: Some(span),
            },
            ShellError::PluginFailedToLoad { msg } => LabeledError {
                label: "Plugin failed to load".into(),
                msg,
                span: None,
            },
            ShellError::PluginFailedToEncode { msg } => LabeledError {
                label: "Plugin failed to encode".into(),
                msg,
                span: None,
            },
            ShellError::PluginFailedToDecode { msg } => LabeledError {
                label: "Plugin failed to decode".into(),
                msg,
                span: None,
            },
            err => LabeledError {
                label: format!("Error - Add to LabeledError From<ShellError>: {err:?}"),
                msg: err.to_string(),
                span: None,
            },
        }
    }
}

/// Response to a [PluginCall]
///
/// Note: exported for internal use, not public.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[doc(hidden)]
pub enum PluginCallResponse {
    Error(LabeledError),
    Signature(Vec<PluginSignature>),
    Empty,
    Value(Box<Value>),
    PluginData(String, PluginData),
    ListStream,
    ExternalStream(ExternalStreamInfo),
}

/// Information received from the plugin
///
/// Note: exported for internal use, not public.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[doc(hidden)]
pub enum PluginOutput {
    CallResponse(PluginCallResponse),
    StreamData(StreamData),
}
