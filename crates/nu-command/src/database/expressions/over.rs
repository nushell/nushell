use crate::database::values::dsl::ExprDb;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use sqlparser::ast::{Expr, WindowSpec};

#[derive(Clone)]
pub struct OverExpr;

impl Command for OverExpr {
    fn name(&self) -> &str {
        "over"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "partition-by",
                SyntaxShape::Any,
                "columns to partition the window function",
            )
            .input_type(Type::Custom("db-expression".into()))
            .output_type(Type::Custom("db-expression".into()))
            .category(Category::Custom("db-expression".into()))
    }

    fn usage(&self) -> &str {
        "Adds a partition to an expression function"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Adds a partition to a function expression",
            example: "fn avg col_a | over col_b | into nu",
            result: Some(Value::Record {
                cols: vec![
                    "name".into(),
                    "args".into(),
                    "over".into(),
                    "distinct".into(),
                ],
                vals: vec![
                    Value::String {
                        val: "avg".into(),
                        span: Span::test_data(),
                    },
                    Value::List {
                        vals: vec![Value::String {
                            val: "col_a".into(),
                            span: Span::test_data(),
                        }],
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "Some(WindowSpec { partition_by: [Identifier(Ident { value: \"col_b\", quote_style: None })], order_by: [], window_frame: None })".into(),
                        span: Span::test_data(),
                    },
                    Value::Bool {
                        val: false,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        },
            Example {
                description: "orders query by a column",
                example: r#"open db.mysql
    | into db
    | select (fn lead col_a | over col_b)
    | from table_a
    | describe"#,
                result: Some(Value::Record {
                    cols: vec!["connection".into(), "query".into()],
                    vals: vec![
                        Value::String {
                            val: "db.mysql".into(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "SELECT lead(col_a) OVER (PARTITION BY col_b) FROM table_a".into(),
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "over", "expression"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::List {
            vals,
            span: call.head,
        };
        let partitions = ExprDb::extract_exprs(value)?;

        let mut expression = ExprDb::try_from_pipeline(input, call.head)?;
        match expression.as_mut() {
            Expr::Function(function) => {
                function.over = Some(WindowSpec {
                    partition_by: partitions,
                    order_by: Vec::new(),
                    window_frame: None,
                });
            }
            s => {
                return Err(ShellError::GenericError(
                    "Expression doesnt define a function".into(),
                    format!("Expected an expression with a function. Got {}", s),
                    Some(call.head),
                    None,
                    Vec::new(),
                ))
            }
        };

        Ok(expression.into_value(call.head).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::super::super::commands::{FromDb, ProjectionDb};
    use super::super::FunctionExpr;
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![
            Box::new(OverExpr {}),
            Box::new(FunctionExpr {}),
            Box::new(ProjectionDb {}),
            Box::new(FromDb {}),
        ])
    }
}
