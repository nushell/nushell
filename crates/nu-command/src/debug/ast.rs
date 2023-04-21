use nu_engine::CallExt;
use nu_parser::parse;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Type, Value,
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
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
            .required(
                "pipeline",
                SyntaxShape::String,
                "the pipeline to print the ast for",
            )
            .switch("json", "serialize to json", Some('j'))
            .allow_variants_without_examples(true)
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
        let to_json = call.has_flag("json");
        let mut working_set = StateWorkingSet::new(engine_state);
        let block_output = parse(&mut working_set, None, pipeline.item.as_bytes(), false);
        let block_span = match &block_output.span {
            Some(span) => span,
            None => &pipeline.span,
        };
        if to_json {
            let block_json = match serde_json::to_string_pretty(&block_output) {
                Ok(json) => json,
                Err(e) => Err(ShellError::CantConvert {
                    to_type: "string".to_string(),
                    from_type: "block".to_string(),
                    span: *block_span,
                    help: Some(format!(
                        "Error: {e}\nCan't convert {block_output:?} to string"
                    )),
                })?,
            };
            Ok(Value::String {
                val: block_json,
                span: pipeline.span,
            }
            .into_pipeline_data())
        } else {
            Ok(Value::String {
                val: format!("{block_output:#?}"),
                span: pipeline.span,
            }
            .into_pipeline_data())
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print the ast of a string",
                example: "ast 'hello'",
                result: None,
            },
            Example {
                description: "Print the ast of a pipeline",
                example: "ast 'ls | where name =~ README'",
                result: None,
            },
            Example {
                description: "Print the ast of a pipeline with an error",
                example: "ast 'for x in 1..10 { echo $x '",
                result: None,
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
