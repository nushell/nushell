mod evaluated_call;
mod plugin_custom_value;
mod plugin_data;

pub use evaluated_call::EvaluatedCall;
use nu_protocol::{ShellError, Signature, Span, Value};
pub use plugin_custom_value::PluginCustomValue;
pub use plugin_data::PluginData;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CallInfo {
    pub name: String,
    pub call: EvaluatedCall,
    pub input: CallInput,
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

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct LabeledError {
    pub label: String,
    pub msg: String,
    pub span: Option<Span>,
}

impl From<LabeledError> for ShellError {
    fn from(error: LabeledError) -> Self {
        match error.span {
            Some(span) => {
                ShellError::GenericError(error.label, error.msg, Some(span), None, Vec::new())
            }
            None => ShellError::GenericError(
                error.label,
                "".to_string(),
                None,
                Some(error.msg),
                Vec::new(),
            ),
        }
    }
}

impl From<ShellError> for LabeledError {
    fn from(error: ShellError) -> Self {
        match error {
            ShellError::GenericError(label, msg, span, _help, _related) => {
                LabeledError { label, msg, span }
            }
            ShellError::CantConvert(expected, input, span, _help) => LabeledError {
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
    PluginData(String, PluginData),
}
