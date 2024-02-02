mod evaluated_call;
mod plugin_custom_value;
mod plugin_data;

pub use evaluated_call::EvaluatedCall;
use nu_protocol::{PluginSignature, ShellError, Span, Value};
pub use plugin_custom_value::PluginCustomValue;
pub use plugin_data::PluginData;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CallInfo {
    pub name: String,
    pub call: EvaluatedCall,
    pub input: CallInput,
    pub config: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum CallInput {
    Value(Value),
    Data(PluginData),
}

// Information sent to the plugin
#[derive(Serialize, Deserialize, Debug)]
pub enum PluginCall {
    Signature,
    CallInfo(CallInfo),
    CollapseCustomValue(PluginData),
}

/// An error message with debugging information that can be passed to Nushell from the plugin
///
/// The `LabeledError` struct is a structured error message that can be returned from
/// a [Plugin](crate::Plugin)'s [`run`](crate::Plugin::run()) method. It contains
/// the error message along with optional [Span] data to support highlighting in the
/// shell.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
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
                label: "Error - Add to LabeledError From<ShellError>".into(),
                msg: err.to_string(),
                span: None,
            },
        }
    }
}

// Information received from the plugin
// Needs to be public to communicate with nu-parser but not typically
// used by Plugin authors
#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub enum PluginResponse {
    Error(LabeledError),
    Signature(Vec<PluginSignature>),
    Value(Box<Value>),
    PluginData(String, PluginData),
}
