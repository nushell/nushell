use nu_engine::{
    command_prelude::*, find_in_dirs_env, get_dirs_var_from_call, get_eval_block_with_early_return,
    redirect_env,
};
use nu_protocol::{
    BlockId,
    engine::CommandType,
    shell_error::{self, io::IoError},
};
use std::path::PathBuf;

/// Source a file for environment variables.
#[derive(Clone)]
pub struct SourceEnv;

impl Command for SourceEnv {
    fn name(&self) -> &str {
        "source-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("source-env")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "filename",
                SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::Nothing]), // type is string to avoid automatically canonicalizing the path
                "The filepath to the script file to source the environment from (`null` for no-op).",
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Source the environment from a source file into the current environment."
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if call.get_parser_info(caller_stack, "noop").is_some() {
            return Ok(PipelineData::empty());
        }

        let source_filename: Spanned<String> = call.req(engine_state, caller_stack, 0)?;

        // Note: this hidden positional is the block_id that corresponded to the 0th position
        // it is put here by the parser
        let block_id: i64 = call.req_parser_info(engine_state, caller_stack, "block_id")?;
        let block_id = BlockId::new(block_id as usize);

        // Set the currently evaluated directory (file-relative PWD)
        let file_path = if let Some(path) = find_in_dirs_env(
            &source_filename.item,
            engine_state,
            caller_stack,
            get_dirs_var_from_call(caller_stack, call),
        )? {
            PathBuf::from(&path)
        } else {
            return Err(ShellError::Io(IoError::new(
                shell_error::io::ErrorKind::FileNotFound,
                source_filename.span,
                PathBuf::from(source_filename.item),
            )));
        };

        if let Some(parent) = file_path.parent() {
            let file_pwd = Value::string(parent.to_string_lossy(), call.head);

            caller_stack.add_env_var("FILE_PWD".to_string(), file_pwd);
        }

        caller_stack.add_env_var(
            "CURRENT_FILE".to_string(),
            Value::string(file_path.to_string_lossy(), call.head),
        );

        // Evaluate the block
        let block = engine_state.get_block(block_id).clone();
        let mut callee_stack = caller_stack
            .gather_captures(engine_state, &block.captures)
            .reset_pipes();

        let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);

        let result = eval_block_with_early_return(engine_state, &mut callee_stack, &block, input)
            .map(|p| p.body);

        // Merge the block's environment to the current stack
        redirect_env(engine_state, caller_stack, &callee_stack);

        // Remove the file-relative PWD
        caller_stack.remove_env_var(engine_state, "FILE_PWD");
        caller_stack.remove_env_var(engine_state, "CURRENT_FILE");

        result
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Sources the environment from foo.nu in the current context",
                example: r#"source-env foo.nu"#,
                result: None,
            },
            Example {
                description: "Sourcing `null` is a no-op.",
                example: r#"source-env null"#,
                result: None,
            },
        ]
    }
}
