use std::sync::atomic;

use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    engine::Closure, LabeledError, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};

use crate::{handle_custom_value::HandleCustomValue, CustomValuePlugin};

pub struct HandleUpdate;

impl SimplePluginCommand for HandleUpdate {
    type Plugin = CustomValuePlugin;

    fn name(&self) -> &str {
        "custom-value handle update"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("HandleCustomValue".into()),
                Type::Custom("HandleCustomValue".into()),
            )
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "the closure to run on the value",
            )
    }

    fn description(&self) -> &str {
        "Update the value in a handle and return a new handle with the result"
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let closure: Spanned<Closure> = call.req(0)?;

        if let Some(handle) = input
            .as_custom_value()?
            .as_any()
            .downcast_ref::<HandleCustomValue>()
        {
            // Find the handle
            let value = plugin
                .handles
                .lock()
                .map_err(|err| LabeledError::new(err.to_string()))?
                .get(&handle.0)
                .cloned();

            if let Some(value) = value {
                // Call the closure with the value
                let new_value = engine.eval_closure(&closure, vec![value.clone()], Some(value))?;

                // Generate an id and store in the plugin.
                let new_id = plugin.counter.fetch_add(1, atomic::Ordering::Relaxed);

                plugin
                    .handles
                    .lock()
                    .map_err(|err| LabeledError::new(err.to_string()))?
                    .insert(new_id, new_value);

                Ok(Value::custom(
                    Box::new(HandleCustomValue(new_id)),
                    call.head,
                ))
            } else {
                Err(LabeledError::new("Handle expired")
                    .with_label("this handle is no longer valid", input.span())
                    .with_help("the plugin may have exited, or there was a bug"))
            }
        } else {
            Err(ShellError::UnsupportedInput {
                msg: "requires HandleCustomValue".into(),
                input: format!("got {}", input.get_type()),
                msg_span: call.head,
                input_span: input.span(),
            }
            .into())
        }
    }
}
