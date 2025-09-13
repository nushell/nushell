use crate::database::{SQLiteDatabase, values::sqlite::nu_value_to_params};
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
        "Query a SQLite database with SQL statements."
    }

    fn extra_description(&self) -> &str {
        "This command is only supported for local or in-memory SQLite databases."
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
            Example {
                description: "Execute a SQL query, selecting a declared JSON(B) column that will automatically be parsed",
                example: r#"stor create -t my_table -c {data: jsonb}
[{data: {name: Albert, age: 40}} {data: {name: Barnaby, age: 54}}] | stor insert -t my_table
stor open | query db "SELECT data FROM my_table WHERE data->>'age' < 45""#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "data" => Value::test_record(
                        record! {
                            "name" => Value::test_string("Albert"),
                            "age" => Value::test_int(40),
                        }
                )})])),
            },
            Example {
                description: "Execute a SQL query selecting a sub-field of a JSON(B) column.
In this case, results must be parsed afterwards because SQLite does not
return declaration types when a JSON(B) column is not directly selected",
                example: r#"stor create -t my_table -c {data: jsonb}
stor insert -t my_table -d {data: {foo: foo, bar: 12, baz: [0 1 2]}}
stor open | query db "SELECT data->'baz' AS baz FROM my_table" | update baz {from json}"#,
                result: Some(Value::test_list(vec![Value::test_record(
                    record! { "baz" =>
                        Value::test_list(vec![
                            Value::test_int(0),
                            Value::test_int(1),
                            Value::test_int(2),
                        ])
                    },
                )])),
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

        let params = nu_value_to_params(engine_state, params_value, call.head)?;

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
