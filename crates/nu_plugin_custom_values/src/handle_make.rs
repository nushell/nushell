use std::sync::atomic;

use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{LabeledError, Signature, Type, Value};

use crate::{handle_custom_value::HandleCustomValue, CustomValuePlugin};

pub struct HandleMake;

impl SimplePluginCommand for HandleMake {
    type Plugin = CustomValuePlugin;

    fn name(&self) -> &str {
        "custom-value handle make"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, Type::Custom("HandleCustomValue".into()))
    }

    fn usage(&self) -> &str {
        "Store a value in plugin memory and return a handle to it"
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        // Generate an id and store in the plugin.
        let new_id = plugin.counter.fetch_add(1, atomic::Ordering::Relaxed);

        plugin
            .handles
            .lock()
            .map_err(|err| LabeledError::new(err.to_string()))?
            .insert(new_id, input.clone());

        Ok(Value::custom(
            Box::new(HandleCustomValue(new_id)),
            call.head,
        ))
    }
}
