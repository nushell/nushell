use std::fs::OpenOptions;

use nu_engine::env::current_dir_str;
use nu_engine::CallExt;
use nu_path::expand_path_with;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct Touch;

impl Command for Touch {
    fn name(&self) -> &str {
        "touch"
    }

    fn signature(&self) -> Signature {
        Signature::build("touch")
            .required(
                "filename",
                SyntaxShape::Filepath,
                "the path of the file you want to create",
            )
            .rest("rest", SyntaxShape::Filepath, "additional files to create")
            .category(Category::FileSystem)
    }

    fn usage(&self) -> &str {
        "Creates one or more files."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let target: String = call.req(engine_state, stack, 0)?;
        let rest: Vec<String> = call.rest(engine_state, stack, 1)?;

        for (index, item) in vec![target].into_iter().chain(rest).enumerate() {
            let path = expand_path_with(&item, current_dir_str(engine_state, stack)?);
            match OpenOptions::new().write(true).create(true).open(&path) {
                Ok(_) => continue,
                Err(err) => {
                    return Err(ShellError::CreateNotPossible(
                        format!("Failed to create file: {}", err),
                        call.positional[index].span,
                    ));
                }
            }
        }

        Ok(PipelineData::new(call.head))
    }
}
