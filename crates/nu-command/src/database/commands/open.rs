use std::{fs::File, io::Read, path::PathBuf};

use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Value,
};

const SQLITE_MAGIC_BYTES: &[u8] = "SQLite format 3\0".as_bytes();

#[derive(Clone)]
pub struct OpenDb;

impl Command for OpenDb {
    fn name(&self) -> &str {
        "db open"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("query", SyntaxShape::Filepath, "SQLite file to be opened")
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Open a database"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "open"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "",
            example: r#"""#,
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
        let path: Spanned<PathBuf> = call.req(engine_state, stack, 0)?;

        let mut file = File::open(&path.item).map_err(|e| {
            ShellError::GenericError(
                "Error opening file".into(),
                e.to_string(),
                Some(path.span),
                None,
                Vec::new(),
            )
        })?;

        let mut buf: [u8; 16] = [0; 16];
        file.read_exact(&mut buf)
            .map_err(|e| {
                ShellError::GenericError(
                    "Error reading file header".into(),
                    e.to_string(),
                    Some(path.span),
                    None,
                    Vec::new(),
                )
            })
            .and_then(|_| {
                if buf == SQLITE_MAGIC_BYTES {
                    let custom_val = Value::CustomValue {
                        val: Box::new(SQLiteDatabase::new(path.item.as_path())),
                        span: call.head,
                    };

                    Ok(custom_val.into_pipeline_data())
                } else {
                    Err(ShellError::GenericError(
                        "Error reading file".into(),
                        "Not a SQLite file".into(),
                        Some(path.span),
                        None,
                        Vec::new(),
                    ))
                }
            })
    }
}
