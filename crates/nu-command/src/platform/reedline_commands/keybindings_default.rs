use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, Signature, Value,
};
use reedline::get_reedline_default_keybindings;

#[derive(Clone)]
pub struct KeybindingsDefault;

impl Command for KeybindingsDefault {
    fn name(&self) -> &str {
        "keybindings default"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Platform)
    }

    fn usage(&self) -> &str {
        "List default keybindings"
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let records = get_reedline_default_keybindings()
            .into_iter()
            .map(|(mode, modifier, code, event)| {
                let mode = Value::String {
                    val: mode,
                    span: call.head,
                };

                let modifier = Value::String {
                    val: modifier,
                    span: call.head,
                };

                let code = Value::String {
                    val: code,
                    span: call.head,
                };

                let event = Value::String {
                    val: event,
                    span: call.head,
                };

                Value::Record {
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

        Ok(Value::List {
            vals: records,
            span: call.head,
        }
        .into_pipeline_data())
    }
}
