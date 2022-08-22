use std::collections::VecDeque;

use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Mkdir;

impl Command for Mkdir {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir")
            .rest(
                "rest",
                SyntaxShape::Directory,
                "the name(s) of the path(s) to create",
            )
            .switch("show-created-paths", "show the path(s) created.", Some('s'))
            .category(Category::FileSystem)
    }

    fn usage(&self) -> &str {
        "Make directories, creates intermediary directories as required."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["directory", "folder", "create", "make_dirs"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let path = current_dir(engine_state, stack)?;
        let mut directories = call
            .rest::<String>(engine_state, stack, 0)?
            .into_iter()
            .map(|dir| path.join(dir))
            .peekable();

        let show_created_paths = call.has_flag("show-created-paths");
        let mut stream: VecDeque<Value> = VecDeque::new();

        if directories.peek().is_none() {
            return Err(ShellError::MissingParameter(
                "requires directory paths".to_string(),
                call.head,
            ));
        }

        for (i, dir) in directories.enumerate() {
            let span = call
                .positional_nth(i)
                .expect("already checked through directories")
                .span;
            let dir_res = std::fs::create_dir_all(&dir);

            if let Err(reason) = dir_res {
                return Err(ShellError::CreateNotPossible(
                    format!("failed to create directory: {}", reason),
                    call.positional_nth(i)
                        .expect("already checked through directories")
                        .span,
                ));
            }

            if show_created_paths {
                let val = format!("{:}", dir.to_string_lossy());
                stream.push_back(Value::String { val, span });
            }
        }

        Ok(stream
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Make a directory named foo",
                example: "mkdir foo",
                result: None,
            },
            Example {
                description: "Make multiple directories and show the paths created",
                example: "mkdir -s foo/bar foo2",
                result: None,
            },
        ]
    }
}
