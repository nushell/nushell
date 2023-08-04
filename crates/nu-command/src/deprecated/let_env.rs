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
            .input_output_types(vec![(Type::Null, Type::Null)])
            .allow_variants_without_examples(true)
            .optional("var_name", SyntaxShape::String, "variable name")
            .optional(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                "equals sign followed by value",
            )
            .category(Category::Deprecated)
    }

    fn usage(&self) -> &str {
        "`let-env FOO = ...` is deprecated, use `$env.FOO = ...` instead."
    }

    fn run(
        &self,
        _: &EngineState,
        _: &mut Stack,
        call: &Call,
        _: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(nu_protocol::ShellError::DeprecatedCommand(
            self.name().to_string(),
            "$env.<environment variable> = ...".to_owned(),
            call.head,
        ))
    }
}
