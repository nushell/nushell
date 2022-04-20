use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

use crate::database::SQLiteDatabase;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "query db"
    }

    fn signature(&self) -> Signature {
        Signature::build("query db")
            .required(
                "query",
                SyntaxShape::String,
                "SQL to execute against the database",
            )
            .category(Category::Date) // TODO: change category
    }

    fn usage(&self) -> &str {
        "Query a database using SQL."
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let sql: Spanned<String> = call.req(engine_state, stack, 0)?;
        let call_head = call.head;

        input.map(
            move |value| query_input(value, call_head, &sql),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get 1 table out of a SQLite database",
            example: r#"open foo.db | query db "SELECT * FROM Bar""#,
            result: None,
        }]
    }
}

fn query_input(input: Value, call_span: Span, sql: &Spanned<String>) -> Value {
    // this fn has slightly awkward error handling because it needs to jam errors into Value instead of returning a Result
    if let Value::CustomValue {
        val,
        span: input_span,
    } = input
    {
        let sqlite = val.as_any().downcast_ref::<SQLiteDatabase>();

        if let Some(db) = sqlite {
            return match db.query(sql, call_span) {
                Ok(val) => val,
                Err(error) => Value::Error { error },
            };
        }

        return Value::Error {
            error: ShellError::PipelineMismatch(
                "a SQLite database".to_string(),
                call_span,
                input_span,
            ),
        };
    }

    match input.span() {
        Ok(input_span) => Value::Error {
            error: ShellError::PipelineMismatch(
                "a SQLite database".to_string(),
                call_span,
                input_span,
            ),
        },
        Err(err) => Value::Error { error: err },
    }
}
