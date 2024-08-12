use crate::database::{values::sqlite::nu_value_to_params, SQLiteDatabase};
use nu_engine::command_prelude::*;

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
                Some('p'),
            )
            .category(Category::Database)
    }

    fn description(&self) -> &str {
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
                example: r#"stor create -t my_table -c { first: str, second: int }
stor open | query db "INSERT INTO my_table VALUES (?, ?)" -p [hello 123]"#,
                result: None,
            },
            Example {
                description: "Execute a SQL statement with named parameters",
                example: r#"stor create -t my_table -c { first: str, second: int }
stor insert -t my_table -d { first: 'hello', second: '123' }
stor open | query db "SELECT * FROM my_table WHERE second = :search_second" -p { search_second: 123 }"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "first" => Value::test_string("hello"),
                    "second" => Value::test_int(123)
                })])),
            },
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
        let params_value: Value = call
            .get_flag(engine_state, stack, "params")?
            .unwrap_or_else(|| Value::nothing(Span::unknown()));

        let params = nu_value_to_params(params_value)?;

        let db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        db.query(&sql, params, call.head)
            .map(IntoPipelineData::into_pipeline_data)
    }
}

#[cfg(test)]
mod test {
    use crate::{StorCreate, StorInsert, StorOpen};

    use super::*;

    #[ignore = "stor db does not persist changes between pipelines"]
    #[test]
    fn test_examples() {
        use crate::test_examples_with_commands;

        test_examples_with_commands(QueryDb {}, &[&StorOpen, &StorCreate, &StorInsert])
    }
}
