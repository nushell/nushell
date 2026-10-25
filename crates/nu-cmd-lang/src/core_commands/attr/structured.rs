use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrStructured;

impl Command for AttrStructured {
    fn name(&self) -> &str {
        "attr structured"
    }

    // The decode block is a `Block`, not a closure, so const eval can materialize it as a
    // capture-free closure (see `eval_const_call_arg`). Like a `def` body, it sees `$in`,
    // constants, `$env` and, when it runs, the `let` variables declared before the extern.
    // `parse_extern_inner` rejects `mut` variables, as a `def` body does.
    fn signature(&self) -> Signature {
        Signature::build("attr structured")
            .input_output_types(vec![(
                Type::Nothing,
                Type::Record(
                    vec![
                        ("args".into(), Type::list(Type::String)),
                        ("decode".into(), Type::Closure),
                    ]
                    .into(),
                ),
            )])
            .allow_variants_without_examples(true)
            .required(
                "args",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Arguments that make the external command print structured output.",
            )
            .required(
                "decode",
                SyntaxShape::Block,
                "Turns that output, given as `$in`, into data.",
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for getting structured output from an external command."
    }

    fn extra_description(&self) -> &str {
        "When the output of the extern is used by nushell (piped into an internal command or \
collected into a variable), the arguments are added right after the command's name and the \
output is decoded with the block. When the output goes to the terminal, `print`, a file, \
`ignore`, `save`, `tee`, `complete` or another external command (`run-external` included), \
the command runs as written. The same holds when the extern is the last command of a `do` \
block or a custom command. Calling the command with a caret, like `^git status`, always runs \
it as written.

The arguments come before the user's own, so a format flag the user passes can override them \
and break the decode, as in `kubectl get pods -o wide`. Use a caret call for that. `tee` \
passes the command's own output on to the commands after it. When an extern has more than \
one `@structured`, the last one is used.

Like a `def` body, the block can read `$in`, `$env`, constants and the `let` variables \
declared before the extern, but not `mut` variables."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args: Value = call.req(engine_state, stack, 0)?;
        let decode: Value = call.req(engine_state, stack, 1)?;
        Ok(
            Value::record(record! { "args" => args, "decode" => decode }, call.head)
                .into_pipeline_data(),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args: Value = call.req_const(working_set, stack, 0)?;
        let decode: Value = call.req_const(working_set, stack, 1)?;
        Ok(
            Value::record(record! { "args" => args, "decode" => decode }, call.head)
                .into_pipeline_data(),
        )
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get the changed files from `git status --porcelain` as a table.",
                example: r#"@structured [--porcelain] { lines | parse --regex '^(?<status>..) (?<file>.+)$' }
    extern "git status" []
    git status | where status =~ M | get file"#,
                result: None,
            },
            Example {
                description: "Get the installed Python packages from `pip list --format=json` as a table.",
                example: r#"@structured [--format=json] { from json }
    extern "pip list" []
    pip list | where name starts-with num"#,
                result: None,
            },
            Example {
                description: "Get the outdated Homebrew formulae from `brew outdated --json=v2` as a table.",
                example: r#"@structured [--json=v2] { from json | get formulae }
    extern "brew outdated" []
    brew outdated | where pinned == false | get name"#,
                result: None,
            },
            Example {
                description: "Get pods from kubectl as a table when the output is piped into `where`.",
                example: r#"@structured [-o json] { from json | get items }
    extern "kubectl get" [resource: string, --namespace(-n): string]: nothing -> table
    kubectl get pods | where status.phase != Running"#,
                result: None,
            },
        ]
    }
}
