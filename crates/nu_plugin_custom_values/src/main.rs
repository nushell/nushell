mod cool_custom_value;
mod second_custom_value;

use cool_custom_value::CoolCustomValue;
use nu_plugin::{serve_plugin, MsgPackSerializer, Plugin};
use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{Category, ShellError, Signature, Value};
use second_custom_value::SecondCustomValue;

struct CustomValuePlugin;

impl Plugin for CustomValuePlugin {
    fn signature(&self) -> Vec<nu_protocol::Signature> {
        vec![
            Signature::build("custom-value generate")
                .usage("Signature for a plugin that generates a custom value")
                .category(Category::Experimental),
            Signature::build("custom-value generate2")
                .usage("Signature for a plugin that generates a different custom value")
                .category(Category::Experimental),
            Signature::build("custom-value update")
                .usage("Signature for a plugin that updates a custom value")
                .category(Category::Experimental),
        ]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        match name {
            "custom-value generate" => self.generate(call, input),
            "custom-value generate2" => self.generate2(call, input),
            "custom-value update" => self.update(call, input),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            }),
        }
    }
}

impl CustomValuePlugin {
    fn generate(&mut self, call: &EvaluatedCall, _input: &Value) -> Result<Value, LabeledError> {
        Ok(CoolCustomValue::new("abc").into_value(call.head))
    }

    fn generate2(&mut self, call: &EvaluatedCall, _input: &Value) -> Result<Value, LabeledError> {
        Ok(SecondCustomValue::new("xyz").into_value(call.head))
    }

    fn update(&mut self, call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
        if let Ok(mut value) = CoolCustomValue::try_from_value(input) {
            value.cool += "xyz";
            return Ok(value.into_value(call.head));
        }

        if let Ok(mut value) = SecondCustomValue::try_from_value(input) {
            value.something += "abc";
            return Ok(value.into_value(call.head));
        }

        Err(ShellError::CantConvert(
            "cool or second".into(),
            "non-cool and non-second".into(),
            call.head,
            None,
        )
        .into())
    }
}

fn main() {
    serve_plugin(&mut CustomValuePlugin, MsgPackSerializer {})
}
