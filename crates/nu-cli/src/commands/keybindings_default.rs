use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SpannedValue, Type,
};
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
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
    }

    fn usage(&self) -> &str {
        "List default keybindings."
    }

    fn examples(&self) -> Vec<Example> {
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
                let mode = SpannedValue::String {
                    val: mode,
                    span: call.head,
                };

                let modifier = SpannedValue::String {
                    val: modifier,
                    span: call.head,
                };

                let code = SpannedValue::String {
                    val: code,
                    span: call.head,
                };

                let event = SpannedValue::String {
                    val: event,
                    span: call.head,
                };

                SpannedValue::Record {
                    cols: vec![
                        "mode".to_string(),
                        "modifier".to_string(),
                        "code".to_string(),
                        "event".to_string(),
                    ],
                    vals: vec![mode, modifier, code, event],
                    span: call.head,
                }
            })
            .collect();

        Ok(SpannedValue::List {
            vals: records,
            span: call.head,
        }
        .into_pipeline_data())
    }
}
