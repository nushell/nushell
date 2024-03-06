mod cool_custom_value;
mod drop_check;
mod second_custom_value;

use cool_custom_value::CoolCustomValue;
use drop_check::DropCheck;
use second_custom_value::SecondCustomValue;

use nu_plugin::{serve_plugin, EngineInterface, MsgPackSerializer, Plugin};
use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{Category, CustomValue, PluginSignature, ShellError, SyntaxShape, Value};

struct CustomValuePlugin;

impl Plugin for CustomValuePlugin {
    fn signature(&self) -> Vec<nu_protocol::PluginSignature> {
        vec![
            PluginSignature::build("custom-value generate")
                .usage("PluginSignature for a plugin that generates a custom value")
                .category(Category::Experimental),
            PluginSignature::build("custom-value generate2")
                .usage("PluginSignature for a plugin that generates a different custom value")
                .optional(
                    "closure",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    "An optional closure to pass the custom value to",
                )
                .category(Category::Experimental),
            PluginSignature::build("custom-value update")
                .usage("PluginSignature for a plugin that updates a custom value")
                .category(Category::Experimental),
            PluginSignature::build("custom-value update-arg")
                .usage("PluginSignature for a plugin that updates a custom value as an argument")
                .required(
                    "custom_value",
                    SyntaxShape::Any,
                    "the custom value to update",
                )
                .category(Category::Experimental),
            PluginSignature::build("custom-value drop-check")
                .usage("Generates a custom value that prints a message when dropped")
                .required("msg", SyntaxShape::String, "the message to print on drop")
                .category(Category::Experimental),
        ]
    }

    fn run(
        &self,
        name: &str,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        match name {
            "custom-value generate" => self.generate(call, input),
            "custom-value generate2" => self.generate2(engine, call),
            "custom-value update" => self.update(call, input),
            "custom-value update-arg" => self.update(call, &call.req(0)?),
            "custom-value drop-check" => self.drop_check(call),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            }),
        }
    }

    fn custom_value_dropped(
        &self,
        _engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        // This is how we implement our drop behavior for DropCheck.
        if let Some(drop_check) = custom_value.as_any().downcast_ref::<DropCheck>() {
            drop_check.notify();
        }
        Ok(())
    }
}

impl CustomValuePlugin {
    fn generate(&self, call: &EvaluatedCall, _input: &Value) -> Result<Value, LabeledError> {
        Ok(CoolCustomValue::new("abc").into_value(call.head))
    }

    fn generate2(
        &self,
        engine: &EngineInterface,
        call: &EvaluatedCall,
    ) -> Result<Value, LabeledError> {
        let second_custom_value = SecondCustomValue::new("xyz").into_value(call.head);
        // If we were passed a closure, execute that instead
        if let Some(closure) = call.opt(0)? {
            let result = engine.eval_closure(
                &closure,
                vec![second_custom_value.clone()],
                Some(second_custom_value),
            )?;
            Ok(result)
        } else {
            Ok(second_custom_value)
        }
    }

    fn update(&self, call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
        if let Ok(mut value) = CoolCustomValue::try_from_value(input) {
            value.cool += "xyz";
            return Ok(value.into_value(call.head));
        }

        if let Ok(mut value) = SecondCustomValue::try_from_value(input) {
            value.something += "abc";
            return Ok(value.into_value(call.head));
        }

        Err(ShellError::CantConvert {
            to_type: "cool or second".into(),
            from_type: "non-cool and non-second".into(),
            span: call.head,
            help: None,
        }
        .into())
    }

    fn drop_check(&self, call: &EvaluatedCall) -> Result<Value, LabeledError> {
        Ok(DropCheck::new(call.req(0)?).into_value(call.head))
    }
}

fn main() {
    serve_plugin(&CustomValuePlugin, MsgPackSerializer {})
}
