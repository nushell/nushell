use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct LetEnv;

impl Command for LetEnv {
    fn name(&self) -> &str {
        "let-env"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .optional("var_name", SyntaxShape::String, "variable name")
            .optional(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                "equals sign followed by value",
            )
            .category(Category::Removed)
    }

    fn usage(&self) -> &str {
        "`let-env FOO = ...` has been removed, use `$env.FOO = ...` instead."
    }

    fn run(
        &self,
        _: &EngineState,
        _: &mut Stack,
        call: &Call,
        _: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(nu_protocol::ShellError::RemovedCommand {
            removed: self.name().to_string(),
            replacement: "$env.<environment variable> = ...".to_owned(),
            span: call.head,
        })
    }
}
