use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
use nu_path::{canonicalize_with, is_windows_device_path};
use nu_protocol::{BlockId, engine::CommandType, shell_error::io::IoError};

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
                SyntaxShape::OneOf(vec![SyntaxShape::Filepath, SyntaxShape::Nothing]),
                "The filepath to the script file to source (`null` for no-op).",
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
        if call.get_parser_info(stack, "noop").is_some() {
            return Ok(PipelineData::empty());
        }
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
        let file_path = if is_windows_device_path(pb.as_path()) {
            pb.clone()
        } else {
            canonicalize_with(pb.as_path(), cwd).map_err(|err| {
                IoError::new(err.not_found_as(NotFound::File), call.head, pb.clone())
            })?
        };

        // Note: We intentionally left out PROCESS_PATH since it's supposed to
        // to work like argv[0] in C, which is the name of the program being executed.
        // Since we're not executing a program, we don't need to set it.

        // Save the old env vars so we can restore them after the script has ran
        let old_file_pwd = stack.get_env_var(engine_state, "FILE_PWD").cloned();
        let old_current_file = stack.get_env_var(engine_state, "CURRENT_FILE").cloned();

        // Add env vars so they are available to the script
        stack.add_env_var(
            "FILE_PWD".to_string(),
            Value::string(parent.to_string_lossy(), Span::unknown()),
        );
        stack.add_env_var(
            "CURRENT_FILE".to_string(),
            Value::string(file_path.to_string_lossy(), Span::unknown()),
        );

        let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);
        let return_result =
            eval_block_with_early_return(engine_state, stack, &block, input).map(|p| p.body);

        // After the script has ran, restore the old values unless they didn't exist.
        // If they didn't exist prior, remove the env vars
        if let Some(old_file_pwd) = old_file_pwd {
            stack.add_env_var("FILE_PWD".to_string(), old_file_pwd.clone());
        } else {
            stack.remove_env_var(engine_state, "FILE_PWD");
        }
        if let Some(old_current_file) = old_current_file {
            stack.add_env_var("CURRENT_FILE".to_string(), old_current_file.clone());
        } else {
            stack.remove_env_var(engine_state, "CURRENT_FILE");
        }

        return_result
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
            Example {
                description: "Sourcing `null` is a no-op.",
                example: r#"source null"#,
                result: None,
            },
            Example {
                description: "Source can be used with const variables.",
                example: r#"const file = if $nu.is-interactive { "interactive.nu" } else { null }; source $file"#,
                result: None,
            },
        ]
    }
}
