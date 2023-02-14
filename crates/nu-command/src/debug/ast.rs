use nu_engine::CallExt;
use nu_parser::parse;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
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
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pipeline: Spanned<String> = call.req(engine_state, stack, 0)?;
        let mut working_set = StateWorkingSet::new(engine_state);

        let (block_output, error_output) =
            parse(&mut working_set, None, pipeline.item.as_bytes(), false, &[]);
        let block_value = Value::String {
            val: format!("{block_output:#?}"),
            span: pipeline.span,
        };
        let error_value = Value::String {
            val: format!("{error_output:#?}"),
            span: pipeline.span,
        };
        let output_record = Value::Record {
            cols: vec!["block".to_string(), "error".to_string()],
            vals: vec![block_value, error_value],
            span: pipeline.span,
        };
        Ok(output_record.into_pipeline_data())
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
