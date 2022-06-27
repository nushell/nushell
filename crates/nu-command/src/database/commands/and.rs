use crate::database::values::dsl::ExprDb;

use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use sqlparser::ast::{BinaryOperator, Expr, Query, Select, SetExpr, Statement};

#[derive(Clone)]
pub struct AndDb;

impl Command for AndDb {
    fn name(&self) -> &str {
        "and"
    }

    fn usage(&self) -> &str {
        "Includes an AND clause for a query"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("where", SyntaxShape::Any, "Where expression on the table")
            .input_type(Type::Custom("database".into()))
            .output_type(Type::Custom("database".into()))
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "where"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Selects a column from a database with an AND clause",
                example: r#"open db.mysql
    | into db
    | select a
    | from table_1
    | where ((field a) > 1)
    | and ((field b) == 1)
    | describe"#,
                result: Some(Value::Record {
                    cols: vec!["connection".into(), "query".into()],
                    vals: vec![
                        Value::String {
                            val: "db.mysql".into(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "SELECT a FROM table_1 WHERE a > 1 AND b = 1".into(),
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Creates a AND clause combined with an expression AND",
                example: r#"open db.mysql
    | into db
    | select a
    | from table_1
    | where ((field a) > 1 | and ((field a) < 10))
    | and ((field b) == 1)
    | describe"#,
                result: Some(Value::Record {
                    cols: vec!["connection".into(), "query".into()],
                    vals: vec![
                        Value::String {
                            val: "db.mysql".into(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "SELECT a FROM table_1 WHERE (a > 1 AND a < 10) AND b = 1".into(),
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
        let value: Value = call.req(engine_state, stack, 0)?;
        let expr = ExprDb::try_from_value(&value)?.into_native();

        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        match db.statement.as_mut() {
            Some(statement) => match statement {
                Statement::Query(query) => modify_query(query, expr, call.head)?,
                s => {
                    return Err(ShellError::GenericError(
                        "Connection doesn't define a query".into(),
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
}

fn modify_query(query: &mut Box<Query>, expression: Expr, span: Span) -> Result<(), ShellError> {
    match query.body {
        SetExpr::Select(ref mut select) => modify_select(select, expression, span)?,
        _ => {
            return Err(ShellError::GenericError(
                "Query without a select".into(),
                "Missing a WHERE clause before an AND clause".into(),
                Some(span),
                None,
                Vec::new(),
            ))
        }
    };

    Ok(())
}

fn modify_select(select: &mut Box<Select>, expression: Expr, span: Span) -> Result<(), ShellError> {
    let new_expression = match &select.selection {
        Some(expr) => Ok(Expr::BinaryOp {
            left: Box::new(expr.clone()),
            op: BinaryOperator::And,
            right: Box::new(expression),
        }),
        None => Err(ShellError::GenericError(
            "Query without a select".into(),
            "Missing a WHERE clause before an AND clause".into(),
            Some(span),
            None,
            Vec::new(),
        )),
    }?;

    select.as_mut().selection = Some(new_expression);
    Ok(())
}

#[cfg(test)]
mod test {
    use super::super::super::expressions::{AndExpr, FieldExpr};
    use super::super::{FromDb, ProjectionDb, WhereDb};
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![
            Box::new(AndDb {}),
            Box::new(ProjectionDb {}),
            Box::new(FromDb {}),
            Box::new(WhereDb {}),
            Box::new(FieldExpr {}),
            Box::new(AndExpr {}),
        ])
    }
}
