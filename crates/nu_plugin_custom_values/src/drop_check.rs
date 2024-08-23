use crate::CustomValuePlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    record, Category, CustomValue, LabeledError, ShellError, Signature, Span, SyntaxShape, Value,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DropCheckValue {
    pub(crate) msg: String,
}

impl DropCheckValue {
    pub(crate) fn new(msg: String) -> DropCheckValue {
        DropCheckValue { msg }
    }

    pub(crate) fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }

    pub(crate) fn notify(&self) {
        eprintln!("DropCheckValue was dropped: {}", self.msg);
    }
}

#[typetag::serde]
impl CustomValue for DropCheckValue {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        "DropCheckValue".into()
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

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn notify_plugin_on_drop(&self) -> bool {
        // This is what causes Nushell to let us know when the value is dropped
        true
    }
}

pub struct DropCheck;

impl SimplePluginCommand for DropCheck {
    type Plugin = CustomValuePlugin;

    fn name(&self) -> &str {
        "custom-value drop-check"
    }

    fn description(&self) -> &str {
        "Generates a custom value that prints a message when dropped"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("msg", SyntaxShape::String, "the message to print on drop")
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        Ok(DropCheckValue::new(call.req(0)?).into_value(call.head))
    }
}
