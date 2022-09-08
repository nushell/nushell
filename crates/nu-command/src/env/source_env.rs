use std::path::PathBuf;

use nu_engine::{eval_block, find_in_dirs_env, redirect_env, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
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
                SyntaxShape::String, // type is string to avoid automatically canonicalizing the path
                "the filepath to the script file to source the environment from",
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

        // Note: this hidden positional is the block_id that corresponded to the 0th position
        // it is put here by the parser
        let block_id: i64 = call.req(engine_state, caller_stack, 1)?;

        // Set the currently evaluated directory (file-relative PWD)
        let mut parent = if let Some(path) =
            find_in_dirs_env(&source_filename.item, engine_state, caller_stack)?
        {
            PathBuf::from(&path)
        } else {
            return Err(ShellError::FileNotFound(source_filename.span));
        };
        parent.pop();

        let file_pwd = Value::String {
            val: parent.to_string_lossy().to_string(),
            span: call.head,
        };

        caller_stack.add_env_var("FILE_PWD".to_string(), file_pwd);

        // Evaluate the block
        let block = engine_state.get_block(block_id as usize).clone();
        let mut callee_stack = caller_stack.gather_captures(&block.captures);

        let result = eval_block(
            engine_state,
            &mut callee_stack,
            &block,
            input,
            call.redirect_stdout,
            call.redirect_stderr,
        );

        // Merge the block's environment to the current stack
        redirect_env(engine_state, caller_stack, &callee_stack);

        // Remove the file-relative PWD
        caller_stack.remove_env_var(engine_state, "FILE_PWD");

        result
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Sources the environment from foo.nu in the current context",
            example: r#"source-env foo.nu"#,
            result: None,
        }]
    }
}
