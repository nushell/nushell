use crate::database::{SQLiteDatabase, MEMORY_DB};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct StorInit;

impl Command for StorInit {
    fn name(&self) -> &str {
        "stor init"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor init")
            .input_output_types(vec![(
                Type::Nothing,
                Type::Custom("sqlite-in-memory".into()),
            )])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the a message indicating initialization."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "storing", "persist"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        eprintln!("Initializing nudb");
        eprintln!("Here's some things to try:");
        eprintln!("* $db | schema | table -e");
        eprintln!("* $db | query db 'insert into nudb (bool1,int1,float1,str1,datetime1) values (2,200,2.0,'str2','1969-04-17T06:00:00-05:00')'");
        eprintln!("* $db | query db 'select * from nudb'");
        eprintln!("Now imagine all those examples happening as commands, without sql, in our normal nushell pipelines\n");

        // let db = open_connection_in_memory_custom()?;
        // db.last_insert_rowid();
        // dbg!(&db);

        // Just create an empty database with MEMORY_DB and nothing else
        let db = Box::new(SQLiteDatabase::new(std::path::Path::new(MEMORY_DB), None));

        // dbg!(db.clone());
        Ok(Value::custom_value(db, span).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Initialize the in-memory sqlite database",
            example: "stor init",
            result: None,
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StorInit {})
    }
}
