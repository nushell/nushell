use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};

use super::super::SQLiteDatabase;

#[derive(Clone)]
pub struct DescribeDb;

impl Command for DescribeDb {
    fn name(&self) -> &str {
        "describe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("database".into()))
            .output_type(Type::Any)
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Describes connection and query of the DB object"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Describe SQLite database constructed query",
            example: "open foo.db | into db | select col_1 | from table_1 | describe",
            result: Some(Value::Record {
                cols: vec!["connection".into(), "query".into()],
                vals: vec![
                    Value::String {
                        val: "foo.db".into(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "SELECT col_1 FROM table_1".into(),
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "SQLite", "describe"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        Ok(db.describe(call.head).into_pipeline_data())
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
            Box::new(DescribeDb {}),
            Box::new(ProjectionDb {}),
            Box::new(FromDb {}),
        ])
    }
}
