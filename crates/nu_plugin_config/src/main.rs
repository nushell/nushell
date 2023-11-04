use nu_plugin::{serve_plugin, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin};
use nu_protocol::{Category, PluginSignature, Type, Value};

fn main() {
    serve_plugin(&mut Config {}, MsgPackSerializer {})
}

#[derive(Clone)]
struct Config;

impl Plugin for Config {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![
            PluginSignature::build("nu-plugin-config")
                .usage("Show plugin configuration")
                .extra_usage("The configuration is set under $env.config.plugins.config")
                .category(Category::Experimental)
                .search_terms(vec!["example".into(), "configuration".into()])
                .input_output_type(Type::Nothing, Type::Table(vec![])),
            PluginSignature::build("nu-plugin-config child")
                .usage("Show plugin configuration subcommand")
                .category(Category::Experimental)
                .input_output_type(Type::Nothing, Type::Table(vec![])),
        ]
    }

    fn run(
        &mut self,
        name: &str,
        config: &Option<Value>,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let allowed_names: Vec<_> = self
            .signature()
            .iter()
            .map(|ps| ps.sig.name.clone())
            .collect();

        if !allowed_names.contains(&name.into()) {
            return Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            });
        }

        match config {
            Some(config) => Ok(config.clone()),
            None => Err(LabeledError {
                label: "No config sent".into(),
                msg: "Configuration for this plugin was not found in `$env.config.plugins.config`"
                    .into(),
                span: Some(call.head),
            }),
        }
    }
}
