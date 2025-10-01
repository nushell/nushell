use nu_engine::command_prelude::*;
use nu_path::expand_path_with;
use nu_protocol::{
    engine::StateWorkingSet,
    shell_error::{self, io::IoError},
};

#[derive(Clone)]
pub struct PathSelf;

impl Command for PathSelf {
    fn name(&self) -> &str {
        "path self"
    }

    fn signature(&self) -> Signature {
        Signature::build("path self")
            .input_output_type(Type::Nothing, Type::String)
            .allow_variants_without_examples(true)
            .optional(
                "path",
                SyntaxShape::Filepath,
                "Path to get instead of the current file.",
            )
            .category(Category::Path)
    }

    fn description(&self) -> &str {
        "Get the absolute path of the script or module containing this command at parse time."
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::GenericError {
            error: "this command can only run during parse-time".into(),
            msg: "can't run after parse-time".into(),
            span: Some(call.head),
            help: Some("try assigning this command's output to a const variable".into()),
            inner: vec![],
        })
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let path: Option<String> = call.opt_const(working_set, 0)?;
        let cwd = working_set.permanent_state.cwd(None)?;
        let current_file = working_set.files.top().ok_or_else(|| {
            IoError::new_with_additional_context(
                shell_error::io::ErrorKind::FileNotFound,
                call.head,
                None,
                "Couldn't find current file",
            )
        })?;

        let out = if let Some(path) = path {
            let dir = expand_path_with(
                current_file.parent().ok_or_else(|| {
                    IoError::new_with_additional_context(
                        shell_error::io::ErrorKind::FileNotFound,
                        call.head,
                        current_file.to_owned(),
                        "Couldn't find current file's parent.",
                    )
                })?,
                &cwd,
                true,
            );
            Value::string(
                expand_path_with(path, dir, false).to_string_lossy(),
                call.head,
            )
        } else {
            Value::string(
                expand_path_with(current_file, &cwd, true).to_string_lossy(),
                call.head,
            )
        };

        Ok(out.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get the path of the current file",
                example: r#"const current_file = path self"#,
                result: None,
            },
            Example {
                description: "Get the path of the directory containing the current file",
                example: r#"const current_file = path self ."#,
                result: None,
            },
            #[cfg(windows)]
            Example {
                description: "Get the absolute form of a path relative to the current file",
                example: r#"const current_file = path self ..\foo"#,
                result: None,
            },
            #[cfg(not(windows))]
            Example {
                description: "Get the absolute form of a path relative to the current file",
                example: r#"const current_file = path self ../foo"#,
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(PathSelf {})
    }
}
