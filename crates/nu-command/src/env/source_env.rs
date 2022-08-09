use std::collections::HashMap;
use std::path::PathBuf;

use nu_engine::{eval_block, CallExt};
use nu_parser::{find_in_dirs, parse, LIB_DIRS_ENV};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value,
};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct SourceEnv;

impl Command for SourceEnv {
    fn name(&self) -> &str {
        "source-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("source-env")
            .required(
                "filename",
                SyntaxShape::String,
                "the filepath to the script file to source the environment frome",
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Source the environment from a source file into the current environment."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let source_filename: Spanned<String> = call.req(engine_state, caller_stack, 0)?;

        let mut working_set = StateWorkingSet::new(&engine_state);

        let cwd = working_set.get_cwd();

        if let Some(path) =
            find_in_dirs(&source_filename.item, &mut working_set, &cwd, LIB_DIRS_ENV)
        {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let mut engine_state = engine_state.clone();

                let mut path = PathBuf::from(&path);
                path.pop();

                engine_state.add_env_var(
                    "PWD".into(),
                    Value::String {
                        val: path.to_string_lossy().to_string(),
                        span: call.head,
                    },
                );

                let mut working_set = StateWorkingSet::new(&engine_state);

                let (block, _) = parse(&mut working_set, None, content.as_bytes(), false, &[]);
                let mut callee_stack = caller_stack.captures_to_stack(&HashMap::new());

                let result =
                    eval_block(&engine_state, &mut callee_stack, &block, input, true, true);

                // add new env vars from callee to caller
                for (var, value) in callee_stack.get_stack_env_vars() {
                    caller_stack.add_env_var(var, value);
                }
                result
            } else {
                Err(ShellError::FileNotFound(source_filename.span))
            }
        } else {
            Err(ShellError::FileNotFound(source_filename.span))
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Sources the environment from foo.nu in the current context",
            example: r#"source-env foo.nu"#,
            result: None,
        }]
    }
}
