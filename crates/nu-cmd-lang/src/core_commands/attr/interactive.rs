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
        "Use this attribute for a completer that needs stdin and the terminal, such as \
`fzf` or `input list`. During Tab completion, it runs on the line-editor thread. \
Unannotated completion sources cannot read stdin, even when they run before an \
interactive fallback.

Returning `null` or `[]` stops fallback without adding candidates; candidates already \
contributed by earlier sources are kept. Evaluation errors also stop fallback. To decline \
and let later completers run, return `{completions: [], fallback: true}`. A nonempty \
`completions` list with `fallback: true` contributes candidates and also continues to later sources.

The language server skips detected interactive completers and tries later sources instead.

The attribute does not register a completion source. Attach \
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
        _stack: &mut Stack,
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
