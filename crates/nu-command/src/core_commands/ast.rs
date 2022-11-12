use nu_engine::CallExt;
use nu_parser::parse;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct Ast;

impl Command for Ast {
    fn name(&self) -> &str {
        "ast"
    }

    fn usage(&self) -> &str {
        "Print the abstract syntax tree (ast) for a pipeline."
    }

    fn signature(&self) -> Signature {
        Signature::build("ast")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "pipeline",
                SyntaxShape::String,
                "the pipeline to print the ast for",
            )
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let pipeline: Spanned<String> = call.req(engine_state, stack, 0)?;
        let mut working_set = StateWorkingSet::new(engine_state);

        let (output, err) = parse(&mut working_set, None, pipeline.item.as_bytes(), false, &[]);
        eprintln!("output: {:#?}\nerror: {:#?}", output, err);

        Ok(PipelineData::new(head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print the ast of a string",
                example: "ast 'hello'",
                result: Some(Value::nothing(Span::test_data())),
            },
            Example {
                description: "Print the ast of a pipeline",
                example: "ast 'ls | where name =~ README'",
                result: Some(Value::nothing(Span::test_data())),
            },
            Example {
                description: "Print the ast of a pipeline with an error",
                example: "ast 'for x in 1..10 { echo $x '",
                result: Some(Value::nothing(Span::test_data())),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Ast;
        use crate::test_examples;
        test_examples(Ast {})
    }
}
