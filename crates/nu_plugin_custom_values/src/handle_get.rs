use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{LabeledError, ShellError, Signature, Type, Value};

use crate::{handle_custom_value::HandleCustomValue, CustomValuePlugin};

pub struct HandleGet;

impl SimplePluginCommand for HandleGet {
    type Plugin = CustomValuePlugin;

    fn name(&self) -> &str {
        "custom-value handle get"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Custom("HandleCustomValue".into()), Type::Any)
    }

    fn description(&self) -> &str {
        "Get a value previously stored in a handle"
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
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
                Ok(value)
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
