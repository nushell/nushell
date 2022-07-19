mod cool_custom_value;

use cool_custom_value::CoolCustomValue;
use nu_plugin::{serve_plugin, JsonSerializer, Plugin};
use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{Category, Signature, Value};

struct CustomValuePlugin;

impl Plugin for CustomValuePlugin {
    fn signature(&self) -> Vec<nu_protocol::Signature> {
        vec![
            Signature::build("custom-value generate")
                .usage("Signature for a plugin that generates a custom value")
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

    fn update(&mut self, call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
        let mut cool = CoolCustomValue::try_from_value(input)?;

        cool.cool += "xyz";

        Ok(cool.into_value(call.head))
    }
}

fn main() {
    serve_plugin(&mut CustomValuePlugin, JsonSerializer {})
}
