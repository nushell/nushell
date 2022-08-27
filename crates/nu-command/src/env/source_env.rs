use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nu_engine::{current_dir, eval_block, redirect_env, CallExt};
use nu_parser::{parse, LIB_DIRS_ENV};
use nu_path::canonicalize_with;
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

                    // Change currently parsed directory
                    working_set.currently_parsed_cwd = Some(parent.clone());

                    let (block, err) =
                        parse(&mut working_set, None, content.as_bytes(), false, &[]);

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

                new_engine_state.merge_delta(delta)?;

                let mut callee_stack = caller_stack.captures_to_stack(&HashMap::new());

                callee_stack.add_env_var(
                    "PWD".to_string(),
                    Value::String {
                        val: parent.to_string_lossy().to_string(),
                        span: call.head,
                    },
                );

                let result = eval_block(
                    &new_engine_state,
                    &mut callee_stack,
                    &block,
                    input,
                    true,
                    true,
                );

                // add new env vars from callee to caller
                redirect_env(&engine_state, caller_stack, &callee_stack);

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

/// This helper function is used to find files during eval
///
/// First, the actual current working directory is selected as
///   a) the directory of a file currently being parsed
///   b) current working directory (PWD)
///
/// Then, if the file is not found in the actual cwd, NU_LIB_DIRS is checked.
/// If there is a relative path in NU_LIB_DIRS, it is assumed to be relative to the actual cwd
/// determined in the first step.
///
/// Always returns an absolute path
pub fn find_in_dirs_env(
    filename: &str,
    engine_state: &EngineState,
    stack: &Stack,
) -> Result<Option<PathBuf>, ShellError> {
    let cwd = current_dir(engine_state, stack)?;

    // Choose whether to use file-relative or PWD-relative path
    // let actual_cwd = if let Some(currently_parsed_cwd) = &working_set.currently_parsed_cwd {
    //     currently_parsed_cwd.as_path()
    // } else {
    //     Path::new(cwd)
    // };

    if let Ok(p) = canonicalize_with(filename, &cwd) {
        Ok(Some(p))
    } else {
        let path = Path::new(filename);

        if path.is_relative() {
            if let Some(lib_dirs) = stack.get_env_var(engine_state, LIB_DIRS_ENV) {
                if let Ok(dirs) = lib_dirs.as_list() {
                    for lib_dir in dirs {
                        if let Ok(dir) = lib_dir.as_path() {
                            // make sure the dir is absolute path
                            if let Ok(dir_abs) = canonicalize_with(&dir, &cwd) {
                                if let Ok(path) = canonicalize_with(filename, dir_abs) {
                                    return Ok(Some(path));
                                }
                            }
                        }
                    }

                    Ok(None)
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
