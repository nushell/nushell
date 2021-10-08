use std::collections::VecDeque;
use std::env::current_dir;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{ShellError, Signature, SyntaxShape, Value, ValueStream};

pub struct Mkdir;

impl Command for Mkdir {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir")
            .rest(
                "rest",
                SyntaxShape::Filepath,
                "the name(s) of the path(s) to create",
            )
            .switch("show-created-paths", "show the path(s) created.", Some('s'))
    }

    fn usage(&self) -> &str {
        "Make directories, creates intermediary directories as required."
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<Value, ShellError> {
        let path = current_dir()?;
        let mut directories = call
            .rest::<String>(context, 0)?
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
            let span = call.positional[i].span;
            let dir_res = std::fs::create_dir_all(&dir);

            if let Err(reason) = dir_res {
                return Err(ShellError::CreateNotPossible(
                    format!("failed to create directory: {}", reason),
                    call.positional[i].span,
                ));
            }

            if show_created_paths {
                let val = format!("{:}", dir.to_string_lossy());
                stream.push_back(Value::String { val, span });
            }
        }

        let stream = ValueStream::from_stream(stream.into_iter());
        let span = call.head;
        Ok(Value::Stream { stream, span })
    }
}
