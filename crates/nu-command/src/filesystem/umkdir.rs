use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type};

use uu_mkdir::mkdir;

#[derive(Clone)]
pub struct UMkdir;

const IS_RECURSIVE: bool = true;
// This is the same default as Rust's std uses:
// https://doc.rust-lang.org/nightly/std/os/unix/fs/trait.DirBuilderExt.html#tymethod.mode
const DEFAULT_MODE: u32 = 0o777;

impl Command for UMkdir {
    fn name(&self) -> &str {
        "umkdir"
    }

    fn usage(&self) -> &str {
        "Create directories, with intermediary directories if required using uutils/coreutils mkdir."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["directory", "folder", "create", "make_dirs", "coreutils"]
    }

    fn signature(&self) -> Signature {
        Signature::build("umkdir")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .rest(
                "rest",
                SyntaxShape::Directory,
                "the name(s) of the path(s) to create",
            )
            .switch(
                "verbose",
                "print a message for each created directory.",
                Some('v'),
            )
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cwd = current_dir(engine_state, stack)?;
        let mut directories = call
            .rest::<String>(engine_state, stack, 0)?
            .into_iter()
            .map(|dir| nu_path::expand_path_with(dir, &cwd))
            .peekable();

        let is_verbose = call.has_flag("verbose");

        if directories.peek().is_none() {
            return Err(ShellError::MissingParameter {
                param_name: "requires directory paths".to_string(),
                span: call.head,
            });
        }

        for dir in directories {
            if let Err(error) = mkdir(&dir, IS_RECURSIVE, DEFAULT_MODE, is_verbose) {
                return Err(ShellError::GenericError {
                    error: format!("{}", error),
                    msg: format!("{}", error),
                    span: None,
                    help: None,
                    inner: vec![],
                });
            }
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Make a directory named foo",
                example: "umkdir foo",
                result: None,
            },
            Example {
                description: "Make multiple directories and show the paths created",
                example: "umkdir -v foo/bar foo2",
                result: None,
            },
        ]
    }
}
