use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
use nu_path::{absolute_with, is_windows_device_path};
use nu_protocol::{engine::CommandType, ir::ParserInfo, shell_error::io::IoError};
use std::path::{Path, PathBuf};

/// Returns the absolute path of the file the parser read for `source` or `source-env`, or a
/// `FileNotFound` error at `span` if the file is gone by the time the command runs.
pub(crate) fn sourced_file_path(
    engine_state: &EngineState,
    stack: &Stack,
    path: &Path,
    span: Span,
) -> Result<PathBuf, ShellError> {
    let cwd = engine_state.cwd_as_string(Some(stack))?;
    if is_windows_device_path(path) {
        return Ok(path.to_path_buf());
    }
    let file_path =
        absolute_with(path, cwd).map_err(|err| IoError::new(err, span, path.to_path_buf()))?;
    match file_path.try_exists() {
        Ok(true) => Ok(file_path),
        Ok(false) => Err(IoError::new(ErrorKind::FileNotFound, span, path.to_path_buf()).into()),
        Err(err) => Err(IoError::new(err, span, path.to_path_buf()).into()),
    }
}

/// Include a script file in the current scope at parse time and run statements outside of custom commands.
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
        "Includes a script file in the current scope at parse time and runs statements outside of custom commands."
    }

    fn extra_description(&self) -> &str {
        "This command is a parser keyword. The file is read and parsed before the script runs, so the path must be a literal or a constant, not the value of a variable. Definitions in the file become available at parse time, but any other top-level statements in the file are run when `source` is evaluated. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"
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
        let block_id = call.req_parser_info(stack, "block_id", ParserInfo::block_id)?;
        let pb = PathBuf::from(call.req_parser_info(stack, "block_id_name", ParserInfo::path)?);
        let block = engine_state.get_block(block_id).clone();
        let parent = pb.parent().unwrap_or(Path::new(""));
        let file_path = sourced_file_path(engine_state, stack, &pb, call.head)?;

        // Note: We intentionally left out PROCESS_PATH since it's supposed to
        // to work like argv[0] in C, which is the name of the program being executed.
        // Since we're not executing a program, we don't need to set it.

        // Save the old env vars so we can restore them after the script has ran
        let old_file_pwd = stack.get_env_var(engine_state, "FILE_PWD").cloned();
        let old_current_file = stack.get_env_var(engine_state, "CURRENT_FILE").cloned();

        // Add env vars so they are available to the script
        stack.add_env_var(
            "FILE_PWD".to_string(),
            Value::string(parent.to_string_lossy(), call.head),
        );
        stack.add_env_var(
            "CURRENT_FILE".to_string(),
            Value::string(file_path.to_string_lossy(), call.head),
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
                description: "Includes foo.nu in the current scope.",
                example: "source foo.nu",
                result: None,
            },
            Example {
                description: "Includes foo.nu in the current scope and calls the command defined, suppose foo.nu has content: `def say-hi [] { echo 'Hi!' }`.",
                example: "source ./foo.nu; say-hi",
                result: None,
            },
            Example {
                description: "Sourcing `null` is a no-op.",
                example: "source null",
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
