use crate::database::values::dsl::ExprDb;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use sqlparser::ast::{BinaryOperator, Expr};

#[derive(Clone)]
pub struct AndExpr;

impl Command for AndExpr {
    fn name(&self) -> &str {
        "and"
    }

    fn usage(&self) -> &str {
        "Includes an AND clause for an expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("and", SyntaxShape::Any, "AND expression")
            .input_type(Type::Custom("db-expression".into()))
            .output_type(Type::Custom("db-expression".into()))
            .category(Category::Custom("db-expression".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "and", "expression"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates an AND expression",
            example: r#"(field a) > 1 | and ((field a) < 10) | into nu"#,
            result: Some(Value::Record {
                cols: vec!["left".into(), "op".into(), "right".into()],
                vals: vec![
                    Value::Record {
                        cols: vec!["left".into(), "op".into(), "right".into()],
                        vals: vec![
                            Value::Record {
                                cols: vec!["value".into(), "quoted_style".into()],
                                vals: vec![
                                    Value::String {
                                        val: "a".into(),
                                        span: Span::test_data(),
                                    },
                                    Value::String {
                                        val: "None".into(),
                                        span: Span::test_data(),
                                    },
                                ],
                                span: Span::test_data(),
                            },
                            Value::String {
                                val: ">".into(),
                                span: Span::test_data(),
                            },
                            Value::String {
                                val: "1".into(),
                                span: Span::test_data(),
                            },
                        ],
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "AND".into(),
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: vec!["left".into(), "op".into(), "right".into()],
                        vals: vec![
                            Value::Record {
                                cols: vec!["value".into(), "quoted_style".into()],
                                vals: vec![
                                    Value::String {
                                        val: "a".into(),
                                        span: Span::test_data(),
                                    },
                                    Value::String {
                                        val: "None".into(),
                                        span: Span::test_data(),
                                    },
                                ],
                                span: Span::test_data(),
                            },
                            Value::String {
                                val: "<".into(),
                                span: Span::test_data(),
                            },
                            Value::String {
                                val: "10".into(),
                                span: Span::test_data(),
                            },
                        ],
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value: Value = call.req(engine_state, stack, 0)?;
        let expr = ExprDb::try_from_value(&value)?.into_native();

        let expression = ExprDb::try_from_pipeline(input, call.head)?;
        let expression = Expr::BinaryOp {
            left: Box::new(expression.into_native()),
            op: BinaryOperator::And,
            right: Box::new(expr),
        };

        let expression: ExprDb = Expr::Nested(Box::new(expression)).into();
        Ok(expression.into_value(call.head).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::super::FieldExpr;
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![Box::new(AndExpr {}), Box::new(FieldExpr {})])
    }
}
