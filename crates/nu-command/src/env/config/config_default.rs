use nu_engine::command_prelude::*;
use nu_protocol::{Config, IntoValue};

#[derive(Clone)]
pub struct ConfigDefault;

impl Command for ConfigDefault {
    fn name(&self) -> &str {
        "config default"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_types(vec![(Type::Nothing, Type::record())])
    }

    fn description(&self) -> &str {
        "Show the default configuration."
    }

    fn extra_description(&self) -> &str {
        "The defaults nushell itself ships, whatever `$env.config` holds now. For \
`keybindings` and `menus` that is the list of entries nushell adds, such as the Tab \
completion menu, and not reedline's own base bindings (see `keybindings default`)."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["reset", "restore", "defaults", "keybindings", "menus"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Show the name of the first default menu.",
                example: "config default | get menus | first | get name",
                result: Some(Value::test_string("completion_menu")),
            },
            Example {
                description: "Reset the keybindings to the defaults, then add one of your own. \
Assigning the defaults first keeps a config that is sourced again from growing the list.",
                example: "$env.config.keybindings = (config default).keybindings; \
$env.config.keybindings ++= [{ name: my_binding, modifier: control, keycode: char_t, mode: emacs, event: { send: clearscreen } }]",
                result: None,
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(default_config(call.head))
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(default_config(call.head))
    }
}

/// The defaults nushell ships, as a record.
fn default_config(span: Span) -> PipelineData {
    Config::default().into_value(span).into_pipeline_data()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ConfigDefault)
    }
}
