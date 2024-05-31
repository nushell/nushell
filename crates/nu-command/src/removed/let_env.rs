use nu_engine::command_prelude::*;

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
            .optional("var_name", SyntaxShape::String, "Variable name.")
            .optional(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                "Equals sign followed by value.",
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
