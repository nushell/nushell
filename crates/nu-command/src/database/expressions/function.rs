use crate::database::values::dsl::ExprDb;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use sqlparser::ast::{Expr, Function, FunctionArg, FunctionArgExpr, Ident, ObjectName};

#[derive(Clone)]
pub struct FunctionExpr;

impl Command for FunctionExpr {
    fn name(&self) -> &str {
        "fn"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("name", SyntaxShape::String, "function name")
            .switch("distinct", "distict values", Some('d'))
            .rest("arguments", SyntaxShape::Any, "function arguments")
            .input_type(Type::Any)
            .output_type(Type::Custom("db-expression".into()))
            .category(Category::Custom("db-expression".into()))
    }

    fn usage(&self) -> &str {
        "Creates function expression for a select operation"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates a function expression",
                example: "fn count name_1 | into nu",
                result: Some(Value::Record {
                    cols: vec![
                        "name".into(),
                        "args".into(),
                        "over".into(),
                        "distinct".into(),
                    ],
                    vals: vec![
                        Value::String {
                            val: "count".into(),
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![Value::String {
                                val: "name_1".into(),
                                span: Span::test_data(),
                            }],
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "None".into(),
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
    | select (fn lead col_a)
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
                            val: "SELECT lead(col_a) FROM table_a".into(),
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "function", "expression"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let name: String = call.req(engine_state, stack, 0)?;
        let vals: Vec<Value> = call.rest(engine_state, stack, 1)?;
        let value = Value::List {
            vals,
            span: call.head,
        };
        let expressions = ExprDb::extract_exprs(value)?;

        let name: Vec<Ident> = name
            .split('.')
            .map(|part| Ident {
                value: part.to_string(),
                quote_style: None,
            })
            .collect();
        let name = ObjectName(name);

        let args: Vec<FunctionArg> = expressions
            .into_iter()
            .map(|expr| {
                let arg = FunctionArgExpr::Expr(expr);

                FunctionArg::Unnamed(arg)
            })
            .collect();

        let expression: ExprDb = Expr::Function(Function {
            name,
            args,
            over: None,
            distinct: call.has_flag("distinct"),
        })
        .into();

        Ok(expression.into_value(call.head).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::super::super::commands::{FromDb, ProjectionDb};
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![
            Box::new(FunctionExpr {}),
            Box::new(ProjectionDb {}),
            Box::new(FromDb {}),
        ])
    }
}
