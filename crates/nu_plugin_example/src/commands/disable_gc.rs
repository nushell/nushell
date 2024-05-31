use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, Value};

use crate::ExamplePlugin;

pub struct DisableGc;

impl SimplePluginCommand for DisableGc {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example disable-gc"
    }

    fn usage(&self) -> &str {
        "Disable the plugin garbage collector for `example`"
    }

    fn extra_usage(&self) -> &str {
        "\
Plugins are garbage collected by default after a period of inactivity. This
behavior is configurable with `$env.config.plugin_gc.default`, or to change it
specifically for the example plugin, use
`$env.config.plugin_gc.plugins.example`.

This command demonstrates how plugins can control this behavior and disable GC
temporarily if they need to. It is still possible to stop the plugin explicitly
using `plugin stop example`."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("reset", "Turn the garbage collector back on", None)
            .category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example", "gc", "plugin_gc", "garbage"]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let disabled = !call.has_flag("reset")?;
        engine.set_gc_disabled(disabled)?;
        Ok(Value::string(
            format!(
                "The plugin garbage collector for `example` is now *{}*.",
                if disabled { "disabled" } else { "enabled" }
            ),
            call.head,
        ))
    }
}
