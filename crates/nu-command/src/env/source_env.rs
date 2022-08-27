use std::path::{PathBuf};

use nu_engine::{eval_block, redirect_env, CallExt, find_in_dirs_env};
use nu_parser::{parse};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, CliError, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value,
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

        if let Some(path) = find_in_dirs_env(&source_filename.item, engine_state, caller_stack)? {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let mut parent = PathBuf::from(&path);
                parent.pop();

                let mut new_engine_state = engine_state.clone();

                let (block, delta) = {
                    let mut working_set = StateWorkingSet::new(&new_engine_state);

                    // Set the currently parsed directory
                    working_set.currently_parsed_cwd = Some(parent.clone());

                    let (block, err) = parse(&mut working_set, None, content.as_bytes(), true, &[]);

                    if let Some(err) = err {
                        let msg = format!(
                            r#"Found this parser error: {:?}"#,
                            CliError(&err, &working_set)
                        );

                        return Err(ShellError::GenericError(
                            "Failed to parse content".to_string(),
                            "cannot parse this file".to_string(),
                            Some(source_filename.span),
                            Some(msg),
                            vec![],
                        ));
                    } else {
                        (block, working_set.render())
                    }
                };

                // Merge parser changes to a temporary engine state
                new_engine_state.merge_delta(delta)?;

                // Set the currently evaluated directory
                let file_pwd = Value::String {
                    val: parent.to_string_lossy().to_string(),
                    span: call.head,
                };

                caller_stack.add_env_var("FILE_PWD".to_string(), file_pwd);

                // Evaluate the parsed file's block
                let mut callee_stack = caller_stack.gather_captures(&block.captures);

                let result = eval_block(
                    &new_engine_state,
                    &mut callee_stack,
                    &block,
                    input,
                    true,
                    true,
                );

                // Merge the block's environment to the current stack
                redirect_env(engine_state, caller_stack, &callee_stack);

                // Remove the file-relative PWD
                caller_stack.remove_env_var(engine_state, "FILE_PWD");

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
