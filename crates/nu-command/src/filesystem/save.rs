use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{ShellError, Signature, SyntaxShape, Value};

pub struct Save;

impl Command for Save {
    fn name(&self) -> &str {
        "save"
    }

    fn signature(&self) -> Signature {
        Signature::build("save")
            .optional(
                "path",
                SyntaxShape::Filepath,
                "the path to save contents to",
            )
            .switch(
                "raw",
                "treat values as-is rather than auto-converting based on file extension",
                Some('r'),
            )
            .switch("append", "append values rather than overriding", Some('a'))
    }

    fn usage(&self) -> &str {
        "Save the contents of the pipeline to a file."
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        _call: &Call,
        _input: Value,
    ) -> Result<Value, ShellError> {
        unimplemented!();
    }
}
