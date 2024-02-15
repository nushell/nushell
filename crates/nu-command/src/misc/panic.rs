use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct Panic;

impl Command for Panic {
    fn name(&self) -> &str {
        "panic"
    }

    fn usage(&self) -> &str {
        "List the filenames, sizes, and modification times of items in a directory."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("ls")
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
            // LsGlobPattern is similar to string, it won't auto-expand
            // and we use it to track if the user input is quoted.
            .optional("msg", SyntaxShape::String, "The glob pattern to use.")
            .category(Category::Misc)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let maybe_msg: String = call
            .opt(engine_state, stack, 0)?
            .unwrap_or("Panic!".to_string());
        panic!("{}", maybe_msg)
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}
