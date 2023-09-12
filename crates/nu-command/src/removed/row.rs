use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, PipelineData, ShellError, Signature, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split row"
    }

    fn signature(&self) -> Signature {
        Signature::build("split row")
            .input_output_types(vec![
                (Type::String, Type::List(Box::new(Type::String))),
                (
                    Type::List(Box::new(Type::String)),
                    (Type::List(Box::new(Type::String))),
                ),
            ])
            .allow_variants_without_examples(true)
            .required(
                "separator",
                SyntaxShape::String,
                "a character or regex that denotes what separates rows",
            )
            .named(
                "number",
                SyntaxShape::Int,
                "Split into maximum number of items",
                Some('n'),
            )
            .switch("regex", "use regex syntax for separator", Some('r'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Removed command: use `str split` instead"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(nu_protocol::ShellError::RemovedCommand(
            self.name().to_string(),
            "str split".to_owned(),
            call.head,
        ))
    }
}
