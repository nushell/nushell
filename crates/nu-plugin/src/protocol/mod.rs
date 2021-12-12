mod evaluated_call;

pub use evaluated_call::EvaluatedCall;
use nu_protocol::{ShellError, Signature, Span, Value};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CallInfo {
    pub name: String,
    pub call: EvaluatedCall,
    pub input: Value,
}

// Information sent to the plugin
#[derive(Serialize, Deserialize, Debug)]
pub enum PluginCall {
    Signature,
    CallInfo(Box<CallInfo>),
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct LabeledError {
    pub label: String,
    pub msg: String,
    pub span: Option<Span>,
}

impl From<LabeledError> for ShellError {
    fn from(error: LabeledError) -> Self {
        match error.span {
            Some(span) => ShellError::SpannedLabeledError(error.label, error.msg, span),
            None => ShellError::LabeledError(error.label, error.msg),
        }
    }
}

impl From<ShellError> for LabeledError {
    fn from(error: ShellError) -> Self {
        match error {
            ShellError::SpannedLabeledError(label, msg, span) => LabeledError {
                label,
                msg,
                span: Some(span),
            },
            ShellError::LabeledError(label, msg) => LabeledError {
                label,
                msg,
                span: None,
            },
            ShellError::CantConvert(expected, input, span) => LabeledError {
                label: format!("Can't convert to {}", expected),
                msg: format!("can't convert {} to {}", expected, input),
                span: Some(span),
            },
            ShellError::DidYouMean(suggestion, span) => LabeledError {
                label: "Name not found".into(),
                msg: format!("did you mean '{}'", suggestion),
                span: Some(span),
            },
            ShellError::PluginFailedToLoad(msg) => LabeledError {
                label: "Plugin failed to load".into(),
                msg,
                span: None,
            },
            ShellError::PluginFailedToEncode(msg) => LabeledError {
                label: "Plugin failed to encode".into(),
                msg,
                span: None,
            },
            ShellError::PluginFailedToDecode(msg) => LabeledError {
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
#[derive(Serialize, Deserialize)]
pub enum PluginResponse {
    Error(LabeledError),
    Signature(Vec<Signature>),
    Value(Box<Value>),
}
