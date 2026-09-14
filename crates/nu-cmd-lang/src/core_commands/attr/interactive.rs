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
completers that must own the terminal should opt in.

The attribute only chooses where a completer runs; it does not make the command one. Attach \
it as usual, `arg: type@name` or `@complete 'name'`; and the completion engine calls it \
with the `token` record when you press Tab. Called directly it is an ordinary command, so \
declare `token: record`: that way `pick-file foo` is rejected at the call site instead of \
failing deep inside the body on `$token.text`. To try a completer by hand, get a real record \
from `commandline complete --input`."
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
            description: "Drive a terminal picker from a completer, then attach it to an argument.",
            example: "\
                @interactive\n\
                def pick-file [token: record] {\n    \
                    ls | get name | to text | ^fzf --query $token.text | lines\n\
                }\n\n\
                def open-file [path: string@pick-file] { open $path }\
            ",
            result: None,
        }]
    }
}
