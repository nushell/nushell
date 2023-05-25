use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str expand"
    }

    fn usage(&self) -> &str {
        "Expands brace patterns in a string, generating all possible combinations."
    }

    fn signature(&self) -> Signature {
        Signature::build("str expand")
            .input_output_types(vec![(Type::String, Type::List(Box::new(Type::String)))])
            .category(Category::Strings)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: span });
        }
        input.map(
            move |v| {
                let value_span = match v.span() {
                    Err(v) => return Value::Error { error: Box::new(v) },
                    Ok(v) => v,
                };
                match v.as_string() {
                    Ok(s) => str_expand(&s, span),
                    Err(_) => Value::Error {
                        error: Box::new(ShellError::PipelineMismatch {
                            exp_input_type: "string".into(),
                            dst_span: span,
                            src_span: value_span,
                        }),
                    },
                }
            },
            engine_state.ctrlc.clone(),
        )
    }
}

fn str_expand(contents: &str, span: Span) -> Value {
    todo!()
}
