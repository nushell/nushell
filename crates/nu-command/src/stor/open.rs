use crate::database::{MEMORY_DB, SQLiteDatabase};
use nu_engine::command_prelude::*;
use nu_protocol::Signals;

#[derive(Clone)]
pub struct StorOpen;

impl Command for StorOpen {
    fn name(&self) -> &str {
        "stor open"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor open")
            .input_output_types(vec![(
                Type::Nothing,
                Type::Custom("sqlite-in-memory".into()),
            )])
            .allow_variants_without_examples(true)
            .category(Category::Database)
    }

    fn description(&self) -> &str {
        "Opens the in-memory sqlite database."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "storing", "access"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Open the in-memory sqlite database",
            example: "stor open",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // eprintln!("Initializing nudb");
        // eprintln!("Here's some things to try:");
        // eprintln!("* stor open | schema | table -e");
        // eprintln!("* stor open | query db 'insert into nudb (bool1,int1,float1,str1,datetime1) values (2,200,2.0,'str2','1969-04-17T06:00:00-05:00')'");
        // eprintln!("* stor open | query db 'select * from nudb'");
        // eprintln!("Now imagine all those examples happening as commands, without sql, in our normal nushell pipelines\n");

        // TODO: Think about adding the following functionality
        // * stor open --table-name my_table_name
        //   It returns the output of `select * from my_table_name`

        // Just create an empty database with MEMORY_DB and nothing else
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));

        // dbg!(db.clone());
        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StorOpen {})
    }
}
