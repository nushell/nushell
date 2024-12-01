use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
use nu_path::canonicalize_with;
use nu_protocol::{engine::CommandType, BlockId};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct Source;

impl Command for Source {
    fn name(&self) -> &str {
        "source"
    }

    fn signature(&self) -> Signature {
        Signature::build("source")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "filename",
                SyntaxShape::Filepath,
                "The filepath to the script file to source.",
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Runs a script file in the current context."
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
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Note: two hidden positionals are used here that are injected by the parser:
        // 1. The block_id that corresponded to the 0th position
        // 2. The block_id_name that corresponded to the file name at the 0th position
        let block_id: i64 = call.req_parser_info(engine_state, stack, "block_id")?;
        let block_id_name: String = call.req_parser_info(engine_state, stack, "block_id_name")?;
        let block_id = BlockId::new(block_id as usize);
        let block = engine_state.get_block(block_id).clone();
        let cwd = engine_state.cwd_as_string(Some(stack))?;
        let pb = std::path::PathBuf::from(block_id_name);
        let parent = pb.parent().unwrap_or(std::path::Path::new(""));
        let file_path =
            canonicalize_with(pb.as_path(), cwd).map_err(|err| ShellError::FileNotFoundCustom {
                msg: format!("Could not access file '{}': {err}", pb.as_path().display()),
                span: Span::unknown(),
            })?;

        let process_path = match pb.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => "unknown".to_string(),
        };

        // Add env vars so they are available to the script
        stack.add_env_var(
            "FILE_PWD".to_string(),
            Value::string(parent.to_string_lossy(), Span::unknown()),
        );
        stack.add_env_var(
            "CURRENT_FILE".to_string(),
            Value::string(file_path.to_string_lossy(), Span::unknown()),
        );
        stack.add_env_var(
            "PROCESS_PATH".to_string(),
            Value::string(process_path, Span::unknown()),
        );

        let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);

        let val = eval_block_with_early_return(engine_state, stack, &block, input);

        // After the script has ran, remove the env vars
        stack.remove_env_var(engine_state, "FILE_PWD");
        stack.remove_env_var(engine_state, "CURRENT_FILE");
        stack.remove_env_var(engine_state, "PROCESS_PATH");

        val
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Runs foo.nu in the current context",
                example: r#"source foo.nu"#,
                result: None,
            },
            Example {
                description: "Runs foo.nu in current context and call the command defined, suppose foo.nu has content: `def say-hi [] { echo 'Hi!' }`",
                example: r#"source ./foo.nu; say-hi"#,
                result: None,
            },
        ]
    }
}
