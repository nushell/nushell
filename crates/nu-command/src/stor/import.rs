use crate::database::{MEMORY_DB, SQLiteDatabase, get_shared_mem_conn};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;

#[derive(Clone)]
pub struct StorImport;

impl Command for StorImport {
    fn name(&self) -> &str {
        "stor import"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor import")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .required_named(
                "file-name",
                SyntaxShape::String,
                "File name to import the sqlite in-memory database from.",
                Some('f'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Database)
    }

    fn description(&self) -> &str {
        "Import a sqlite database file into the in-memory sqlite database."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "open", "database", "restore", "file"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Import a sqlite database file into the in-memory sqlite database",
            example: "stor import --file-name nudb.sqlite",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let file_name_opt: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "file-name")?;
        let file_name = match file_name_opt {
            Some(file_name) => file_name,
            None => {
                return Err(ShellError::MissingParameter {
                    param_name: "please supply a file name with the --file-name parameter".into(),
                    span,
                });
            }
        };

        // `Connection::restore` opens the source with `OpenFlags::default()`, which includes
        // `SQLITE_OPEN_CREATE`, so a missing path is created as an empty database and then
        // restored over the in-memory one, discarding its contents without reporting an
        // error. Reject the path up front so that cannot happen.
        let path = std::path::PathBuf::from(&file_name.item);
        match path.try_exists() {
            Ok(true) => {}
            Ok(false) => {
                return Err(IoError::new(ErrorKind::FileNotFound, file_name.span, path).into());
            }
            Err(err) => return Err(IoError::new(err, file_name.span, path).into()),
        }

        let mut conn = get_shared_mem_conn()?;
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            engine_state.signals().clone(),
        ));
        db.restore_database_from_file(&mut conn, file_name.item)
            .map_err(|err| {
                ShellError::Generic(GenericError::new_internal(
                    "Failed to open SQLite connection to the in-memory database from import",
                    err.to_string(),
                ))
            })?;

        Ok(Value::custom(db, span).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(StorImport)
    }
}
