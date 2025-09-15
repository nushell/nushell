use nu_engine::command_prelude::*;
use reedline::get_reedline_default_keybindings;

#[derive(Clone)]
pub struct KeybindingsDefault;

impl Command for KeybindingsDefault {
    fn name(&self) -> &str {
        "keybindings default"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn description(&self) -> &str {
        "List default keybindings."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Get list with default keybindings",
            example: "keybindings default",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let records = get_reedline_default_keybindings()
            .into_iter()
            .map(|(mode, modifier, code, event)| {
                Value::record(
                    record! {
                        "mode" => Value::string(mode, call.head),
                        "modifier" => Value::string(modifier, call.head),
                        "code" => Value::string(code, call.head),
                        "event" => Value::string(event, call.head),
                    },
                    call.head,
                )
            })
            .collect();

        Ok(Value::list(records, call.head).into_pipeline_data())
    }
}
