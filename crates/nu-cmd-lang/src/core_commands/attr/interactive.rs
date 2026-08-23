use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrInteractive;

impl Command for AttrInteractive {
    fn name(&self) -> &str {
        "attr interactive"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr interactive")
            .input_output_type(Type::Nothing, Type::Bool)
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute marking a completer as interactive."
    }

    fn extra_description(&self) -> &str {
        "An interactive completer runs on the line-editor thread, with the terminal to \
itself, instead of on the background completion worker. This lets it drive a terminal \
picker such as `fzf` or `input list`, which need stdin and the TTY. Every other completer \
stays on the worker, so it never blocks the line editor and its result can be cached; only \
completers that must own the terminal should opt in."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::bool(true, call.head).into_pipeline_data())
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::bool(true, call.head).into_pipeline_data())
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Mark a completer as interactive so it can drive a terminal picker.",
            example: "@interactive\ndef my-completer [token] { ls | get name | to text | ^fzf --query $token.text }",
            result: None,
        }]
    }
}
