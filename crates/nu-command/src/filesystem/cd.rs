use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, Signature, SyntaxShape};

#[derive(Clone)]
pub struct Cd;

impl Command for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn usage(&self) -> &str {
        "Change directory."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cd")
            .optional("path", SyntaxShape::Filepath, "the path to change to")
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let path: Option<String> = call.opt(engine_state, stack, 0)?;

        let path = match path {
            Some(path) => {
                let path = nu_path::expand_path(path);
                path.to_string_lossy().to_string()
            }
            None => {
                let path = nu_path::expand_tilde("~");
                path.to_string_lossy().to_string()
            }
        };
        let _ = std::env::set_current_dir(&path);

        //FIXME: this only changes the current scope, but instead this environment variable
        //should probably be a block that loads the information from the state in the overlay
        stack.add_env_var("PWD".into(), path);
        Ok(PipelineData::new(call.head))
    }
}
