use crate::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use sqlparser::ast::{Ident, SetExpr, Statement, TableAlias, TableFactor};

#[derive(Clone)]
pub struct AliasDb;

impl Command for AliasDb {
    fn name(&self) -> &str {
        "as"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("alias", SyntaxShape::String, "alias name")
            .input_type(Type::Custom("database".into()))
            .output_type(Type::Custom("database".into()))
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Creates an alias for a column selection"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates an alias for a selected table",
                example: r#"open db.mysql
    | into db
    | select a
    | from table_1
    | as t1
    | describe"#,
                result: Some(Value::Record {
                    cols: vec!["connection".into(), "query".into()],
                    vals: vec![
                        Value::String {
                            val: "db.mysql".into(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "SELECT a FROM table_1 AS t1".into(),
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Creates an alias for a derived table",
                example: r#"open db.mysql
    | into db
    | select a
    | from (
        open db.mysql
        | into db
        | select a b
        | from table_a
      )
    | as t1
    | describe"#,
                result: Some(Value::Record {
                    cols: vec!["connection".into(), "query".into()],
                    vals: vec![
                        Value::String {
                            val: "db.mysql".into(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "SELECT a FROM (SELECT a, b FROM table_a) AS t1".into(),
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "alias", "column"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let alias: String = call.req(engine_state, stack, 0)?;

        let db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        alias_db(db, alias, call)
    }
}

fn alias_db(
    mut db: SQLiteDatabase,
    new_alias: String,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    match db.statement.as_mut() {
        None => Err(ShellError::GenericError(
            "Error creating alias".into(),
            "there is no statement defined yet".into(),
            Some(call.head),
            None,
            Vec::new(),
        )),
        Some(statement) => match statement {
            Statement::Query(query) => match &mut query.body {
                SetExpr::Select(select) => {
                    select.as_mut().from.iter_mut().for_each(|table| {
                        let new_alias = Some(TableAlias {
                            name: Ident {
                                value: new_alias.clone(),
                                quote_style: None,
                            },
                            columns: Vec::new(),
                        });

                        if let TableFactor::Table { ref mut alias, .. } = table.relation {
                            *alias = new_alias;
                        } else if let TableFactor::Derived { ref mut alias, .. } = table.relation {
                            *alias = new_alias;
                        } else if let TableFactor::TableFunction { ref mut alias, .. } =
                            table.relation
                        {
                            *alias = new_alias;
                        }
                    });

                    Ok(db.into_value(call.head).into_pipeline_data())
                }
                _ => Err(ShellError::GenericError(
                    "Error creating alias".into(),
                    "Query has no select from defined".into(),
                    Some(call.head),
                    None,
                    Vec::new(),
                )),
            },
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
    }
}

#[cfg(test)]
mod test {
    use super::super::{FromDb, ProjectionDb};
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![
            Box::new(AliasDb {}),
            Box::new(ProjectionDb {}),
            Box::new(FromDb {}),
        ])
    }
}
