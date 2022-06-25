use crate::database::values::dsl::ExprDb;

use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use sqlparser::ast::{SetExpr, Statement};

#[derive(Clone)]
pub struct GroupByDb;

impl Command for GroupByDb {
    fn name(&self) -> &str {
        "group-by"
    }

    fn usage(&self) -> &str {
        "Group by query"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
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
        vec!["database", "select"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "groups by column a and calculates the max",
                example: r#"open db.mysql
    | into db
    | from table_a
    | select (fn max a)
    | group-by a
    | describe"#,
                result: Some(Value::Record {
                    cols: vec!["connection".into(), "query".into()],
                    vals: vec![
                        Value::String {
                            val: "db.mysql".into(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "SELECT max(a) FROM table_a GROUP BY a".into(),
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "groups by column column a and counts records",
                example: r#"open db.mysql
    | into db
    | from table_a
    | select (fn count *)
    | group-by a
    | describe"#,
                result: Some(Value::Record {
                    cols: vec!["connection".into(), "query".into()],
                    vals: vec![
                        Value::String {
                            val: "db.mysql".into(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "SELECT count(*) FROM table_a GROUP BY a".into(),
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
        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::List {
            vals,
            span: call.head,
        };
        let expressions = ExprDb::extract_exprs(value)?;

        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        match db.statement.as_mut() {
            Some(statement) => match statement {
                Statement::Query(ref mut query) => match &mut query.body {
                    SetExpr::Select(ref mut select) => select.group_by = expressions,
                    s => {
                        return Err(ShellError::GenericError(
                            "Connection doesnt define a select".into(),
                            format!("Expected a connection with select query. Got {}", s),
                            Some(call.head),
                            None,
                            Vec::new(),
                        ))
                    }
                },
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
}

#[cfg(test)]
mod test {
    use super::super::super::expressions::{FieldExpr, FunctionExpr, OrExpr};
    use super::super::{FromDb, ProjectionDb, WhereDb};
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![
            Box::new(GroupByDb {}),
            Box::new(ProjectionDb {}),
            Box::new(FunctionExpr {}),
            Box::new(FromDb {}),
            Box::new(WhereDb {}),
            Box::new(FieldExpr {}),
            Box::new(OrExpr {}),
        ])
    }
}
