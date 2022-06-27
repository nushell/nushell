use crate::database::values::dsl::ExprDb;

use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use sqlparser::ast::{Expr, OrderByExpr, Statement};

#[derive(Clone)]
pub struct OrderByDb;

impl Command for OrderByDb {
    fn name(&self) -> &str {
        "order-by"
    }

    fn usage(&self) -> &str {
        "Orders by query"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("ascending", "Order by ascending values", Some('a'))
            .switch("nulls-first", "Show nulls first in order", Some('n'))
            .rest(
                "select",
                SyntaxShape::Any,
                "Select expression(s) on the table",
            )
            .input_type(Type::Custom("database".into()))
            .output_type(Type::Custom("database".into()))
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "order-by"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "orders query by a column",
                example: r#"open db.mysql
    | into db
    | from table_a
    | select a
    | order-by a
    | describe"#,
                result: Some(Value::Record {
                    cols: vec!["connection".into(), "query".into()],
                    vals: vec![
                        Value::String {
                            val: "db.mysql".into(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "SELECT a FROM table_a ORDER BY a".into(),
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "orders query by column a ascending and by column b",
                example: r#"open db.mysql
    | into db
    | from table_a
    | select a
    | order-by a --ascending
    | order-by b
    | describe"#,
                result: Some(Value::Record {
                    cols: vec!["connection".into(), "query".into()],
                    vals: vec![
                        Value::String {
                            val: "db.mysql".into(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "SELECT a FROM table_a ORDER BY a ASC, b".into(),
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let asc = call.has_flag("ascending");
        let nulls_first = call.has_flag("nulls-first");
        let expressions: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let expressions = Value::List {
            vals: expressions,
            span: call.head,
        };
        let expressions = ExprDb::extract_exprs(expressions)?;
        let expressions: Vec<OrderByExpr> = expressions
            .into_iter()
            .map(|expr| OrderByExpr {
                expr,
                asc: if asc { Some(asc) } else { None },
                nulls_first: if nulls_first { Some(nulls_first) } else { None },
            })
            .collect();

        let value = input.into_value(call.head);

        if let Ok(expr) = ExprDb::try_from_value(&value) {
            update_expressions(expr, expressions, call)
        } else if let Ok(db) = SQLiteDatabase::try_from_value(value.clone()) {
            update_connection(db, expressions, call)
        } else {
            Err(ShellError::CantConvert(
                "expression or query".into(),
                value.get_type().to_string(),
                value.span()?,
                None,
            ))
        }
    }
}

fn update_expressions(
    mut expr: ExprDb,
    mut expressions: Vec<OrderByExpr>,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    match expr.as_mut() {
        Expr::Function(function) => match &mut function.over {
            Some(over) => over.order_by.append(&mut expressions),
            None => {
                return Err(ShellError::GenericError(
                    "Expression doesnt define a partition to order".into(),
                    "Expected an expression with partition".into(),
                    Some(call.head),
                    None,
                    Vec::new(),
                ))
            }
        },
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

    Ok(expr.into_value(call.head).into_pipeline_data())
}

fn update_connection(
    mut db: SQLiteDatabase,
    mut expressions: Vec<OrderByExpr>,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    match db.statement.as_mut() {
        Some(statement) => match statement {
            Statement::Query(query) => {
                query.order_by.append(&mut expressions);
            }
            s => {
                return Err(ShellError::GenericError(
                    "Connection doesnt define a query".into(),
                    format!("Expected a connection with query. Got {}", s),
                    Some(call.head),
                    None,
                    Vec::new(),
                ))
            }
        },
        None => {
            return Err(ShellError::GenericError(
                "Connection without statement".into(),
                "The connection needs a statement defined".into(),
                Some(call.head),
                None,
                Vec::new(),
            ))
        }
    };

    Ok(db.into_value(call.head).into_pipeline_data())
}

#[cfg(test)]
mod test {
    use super::super::super::expressions::{FieldExpr, OrExpr};
    use super::super::{FromDb, ProjectionDb, WhereDb};
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![
            Box::new(OrderByDb {}),
            Box::new(ProjectionDb {}),
            Box::new(FromDb {}),
            Box::new(WhereDb {}),
            Box::new(FieldExpr {}),
            Box::new(OrExpr {}),
        ])
    }
}
