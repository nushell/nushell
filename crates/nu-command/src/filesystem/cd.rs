use nu_engine::env::current_dir_str;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, Signature, SyntaxShape, Value};

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
        let path_val: Option<Value> = call.opt(engine_state, stack, 0)?;

        let (path, span) = match path_val {
            Some(v) => {
                let path = nu_path::canonicalize_with(
                    v.as_string()?,
                    current_dir_str(engine_state, stack)?,
                )?;
                (path.to_string_lossy().to_string(), v.span()?)
            }
            None => {
                let path = nu_path::expand_tilde("~");
                (path.to_string_lossy().to_string(), call.head)
            }
        };

        //FIXME: this only changes the current scope, but instead this environment variable
        //should probably be a block that loads the information from the state in the overlay
        stack.add_env_var("PWD".into(), Value::String { val: path, span });
        Ok(PipelineData::new(call.head))
    }
}
