use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, Value};

use crate::Example;

pub struct DisableGc;

impl SimplePluginCommand for DisableGc {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("example disable-gc")
            .usage("Disable the plugin garbage collector for `example`")
            .extra_usage(
                "\
Plugins are garbage collected by default after a period of inactivity. This
behavior is configurable with `$env.config.plugin_gc.default`, or to change it
specifically for the example plugin, use
`$env.config.plugin_gc.plugins.example`.

This command demonstrates how plugins can control this behavior and disable GC
temporarily if they need to. It is still possible to stop the plugin explicitly
using `plugin stop example`.",
            )
            .search_terms(vec![
                "example".into(),
                "gc".into(),
                "plugin_gc".into(),
                "garbage".into(),
            ])
            .switch("reset", "Turn the garbage collector back on", None)
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _plugin: &Example,
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
