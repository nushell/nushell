use nu_protocol::{record, CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DropCheck {
    pub(crate) msg: String,
}

impl DropCheck {
    pub(crate) fn new(msg: String) -> DropCheck {
        DropCheck { msg }
    }

    pub(crate) fn into_value(self, span: Span) -> Value {
        Value::custom_value(Box::new(self), span)
    }

    pub(crate) fn notify(&self) {
        eprintln!("DropCheck was dropped: {}", self.msg);
    }
}

#[typetag::serde]
impl CustomValue for DropCheck {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn value_string(&self) -> String {
        "DropCheck".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::record(
            record! {
                "msg" => Value::string(&self.msg, span)
            },
            span,
        ))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn notify_plugin_on_drop(&self) -> bool {
        // This is what causes Nushell to let us know when the value is dropped
        true
    }
}
