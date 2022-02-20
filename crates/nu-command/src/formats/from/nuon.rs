use nu_engine::eval_expression;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};
#[derive(Clone)]
pub struct FromNuon;

impl Command for FromNuon {
    fn name(&self) -> &str {
        "from nuon"
    }

    fn usage(&self) -> &str {
        "Convert from nuon to structured data"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from nuon").category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "'{ a:1 }' | from nuon",
                description: "Converts json formatted string to table",
                result: Some(Value::Record {
                    cols: vec!["a".to_string()],
                    vals: vec![Value::Int {
                        val: 1,
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "'{ a:1, b: [1, 2] }' | from nuon",
                description: "Converts json formatted string to table",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![
                        Value::Int {
                            val: 1,
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![
                                Value::Int {
                                    val: 1,
                                    span: Span::test_data(),
                                },
                                Value::Int {
                                    val: 2,
                                    span: Span::test_data(),
                                },
                            ],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let config = stack.get_config().unwrap_or_default();
        let string_input = input.collect_string("", &config)?;

        {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            let _ = working_set.add_file("nuon file".to_string(), string_input.as_bytes());

            // let span = Span {
            //     start: 0,
            //     end: string_input.len(),
            // };

            let mut error = None;

            let (lexed, err) =
                nu_parser::lex(string_input.as_bytes(), 0, &[b'\n', b'\r'], &[], true);
            error = error.or(err);

            let (lite_block, err) = nu_parser::lite_parse(&lexed);
            error = error.or(err);

            let (block, err) = nu_parser::parse_block(&mut working_set, &lite_block, true);
            error = error.or(err);

            if block.pipelines.get(1).is_some() {
                return Err(ShellError::SpannedLabeledError(
                    "error when loading".into(),
                    "excess values when loading".into(),
                    head,
                ));
            }

            let expr = if let Some(pipeline) = block.pipelines.get(0) {
                if pipeline.expressions.get(1).is_some() {
                    return Err(ShellError::SpannedLabeledError(
                        "error when loading".into(),
                        "detected a pipeline in nuon file".into(),
                        head,
                    ));
                }

                if let Some(expr) = pipeline.expressions.get(0) {
                    expr.clone()
                } else {
                    Expression {
                        expr: Expr::Nothing,
                        span: head,
                        custom_completion: None,
                        ty: Type::Nothing,
                    }
                }
            } else {
                Expression {
                    expr: Expr::Nothing,
                    span: head,
                    custom_completion: None,
                    ty: Type::Nothing,
                }
            };

            if let Some(err) = error {
                return Err(ShellError::SpannedLabeledError(
                    "error when loading".into(),
                    err.to_string(),
                    head,
                ));
            }
            let mut stack = Stack::new();
            let result = eval_expression(&engine_state, &mut stack, &expr);

            match result {
                Ok(result) => Ok(result.into_pipeline_data()),
                Err(err) => Err(ShellError::SpannedLabeledError(
                    "error when loading".into(),
                    err.to_string(),
                    head,
                )),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromNuon {})
    }
}
