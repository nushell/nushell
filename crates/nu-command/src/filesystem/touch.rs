use std::fs::OpenOptions;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{PipelineData, ShellError, Signature, SyntaxShape, Value};

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
    }

    fn usage(&self) -> &str {
        "Creates one or more files."
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let target: String = call.req(context, 0)?;
        let rest: Vec<String> = call.rest(context, 1)?;

        for (index, item) in vec![target].into_iter().chain(rest).enumerate() {
            match OpenOptions::new().write(true).create(true).open(&item) {
                Ok(_) => continue,
                Err(err) => {
                    return Err(ShellError::CreateNotPossible(
                        format!("Failed to create file: {}", err),
                        call.positional[index].span,
                    ));
                }
            }
        }

        Ok(PipelineData::new())
    }
}
