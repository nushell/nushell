use nu_engine::CallExt;
use nu_protocol::{
    ast::Call, engine::{Command, EngineState, Stack}, Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value
};

use crate::database::values::sqlite::nu_value_to_params;

use super::super::SQLiteDatabase;

#[derive(Clone)]
pub struct QueryDb;

impl Command for QueryDb {
    fn name(&self) -> &str {
        "query db"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "SQL",
                SyntaxShape::String,
                "SQL to execute against the database.",
            )
            .named(
                "params",
                // TODO: Use SyntaxShape::OneOf with Records and Lists, when Lists no longer break inside OneOf
                SyntaxShape::Any,
                "List of parameters for the SQL statement",
                Some('p')
            )
            .category(Category::Database)
    }

    fn usage(&self) -> &str {
        "Query a database using SQL."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Execute SQL against a SQLite database",
                example: r#"open foo.db | query db "SELECT * FROM Bar""#,
                result: None,
            },
            Example {
                description: "Execute a SQL statement with parameters",
                example: r#"stor open | query db "INSERT INTO my_table VALUES (?, ?)" -p [hello 123]"#,
                result: None,
            },
            Example {
                description: "Execute a SQL statement with named parameters",
                example: r#"stor open | query db "INSERT INTO my_table VALUES (:first, :second)" -p { ":first": "hello", ":second": 123 }"#,
                result: None,
            }
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "SQLite"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let sql: Spanned<String> = call.req(engine_state, stack, 0)?;
        let params_value: Value = call.get_flag(engine_state, stack, "params")?
            .unwrap_or_else(|| Value::nothing(Span::unknown()));

        let params = nu_value_to_params(&params_value)?;

        let db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        db.query(&sql, params, call.head)
            .map(IntoPipelineData::into_pipeline_data)
    }
}
