use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset, Local};

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use uu_touch::{datetime_to_filetime, stat, InputFile, Options};

#[derive(Clone)]
pub struct UTouch;

impl Command for UTouch {
    fn name(&self) -> &str {
        "utouch"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["create", "file"]
    }

    fn signature(&self) -> Signature {
        Signature::build("touch")
            .input_output_types(vec![ (Type::Nothing, Type::Nothing) ])
            .required(
                "filename",
                SyntaxShape::Filepath,
                "The path of the file you want to create.",
            )
            .named(
                "reference",
                SyntaxShape::Filepath,
                "change the file or directory time to the time of the reference file/directory",
                Some('r'),
            )
            .named(
                "timestamp",
                SyntaxShape::DateTime,
                "use the given time instead of the current time",
                Some('t')
            )
            .switch(
                "modified",
                "change the modification time of the file or directory. If no timestamp, date or reference file/directory is given, the current time is used",
                Some('m'),
            )
            .switch(
                "access",
                "change the access time of the file or directory. If no timestamp, date or reference file/directory is given, the current time is used",
                Some('a'),
            )
            .switch(
                "no-create",
                "do not create the file if it does not exist",
                Some('c'),
            )
            .switch(
                "no-dereference",
                "affect each symbolic link instead of any referenced file (only for systems that can change the timestamps of a symlink)",
                None
            )
            .rest("rest", SyntaxShape::Filepath, "Additional files to create.")
            .category(Category::FileSystem)
    }

    fn usage(&self) -> &str {
        "Creates one or more files."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let change_mtime: bool = call.has_flag(engine_state, stack, "modified")?;
        let change_atime: bool = call.has_flag(engine_state, stack, "access")?;
        let no_create: bool = call.has_flag(engine_state, stack, "no-create")?;
        let no_deref: bool = call.has_flag(engine_state, stack, "no-dereference")?;
        let target: Spanned<String> = call.req(engine_state, stack, 0)?;
        let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 1)?;

        let reference: Option<Spanned<PathBuf>> =
            call.get_flag(engine_state, stack, "reference")?;
        let timestamp: Option<Spanned<DateTime<FixedOffset>>> =
            call.get_flag(engine_state, stack, "timestamp")?;

        let (atime, mtime) = if let Some(timestamp) = timestamp {
            if let Some(reference) = reference {
                return Err(ShellError::IncompatibleParameters {
                    left_message: "timestamp given".to_string(),
                    left_span: timestamp.span,
                    right_message: "reference given".to_string(),
                    right_span: reference.span,
                });
            }
            let filetime = datetime_to_filetime(&timestamp.item);
            (filetime, filetime)
        } else if let Some(reference) = reference {
            let reference_path = Path::new(&reference.item);
            if !reference_path.exists() {
                return Err(ShellError::TypeMismatch {
                    err_message: format!("path provided is invalid: {}", reference_path.display()),
                    span: reference.span,
                });
            }
            stat(reference_path, !no_deref).map_err(|e| ShellError::GenericError {
                error: "couldn't get metadata".to_string(),
                msg: format!("{}", e),
                span: Some(reference.span),
                help: None,
                inner: Vec::new(),
            })?
        } else {
            let now = datetime_to_filetime(&Local::now());
            (now, now)
        };

        for file in vec![target].into_iter().chain(rest) {
            if let Err(err) = uu_touch::touch(
                &InputFile::Path(PathBuf::from(file.item)),
                &Options {
                    no_create,
                    no_deref,
                    atime: if !change_mtime || change_atime {
                        Some(atime)
                    } else {
                        None
                    },
                    mtime: if !change_atime || change_mtime {
                        Some(mtime)
                    } else {
                        None
                    },
                },
            ) {
                return Err(ShellError::GenericError {
                    error: "utouch failed".to_string(),
                    msg: err.to_string(),
                    span: Some(file.span),
                    help: None,
                    inner: Vec::new(),
                });
            }
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates \"fixture.json\"",
                example: "utouch fixture.json",
                result: None,
            },
            Example {
                description: "Creates files a, b and c",
                example: "utouch a b c",
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of "fixture.json" to today's date"#,
                example: "utouch -m fixture.json",
                result: None,
            },
            Example {
                description: "Changes the last modified time of files a, b and c to a date",
                example: r#"utouch -m -d "yesterday" a b c"#,
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of file d and e to "fixture.json"'s last modified time"#,
                example: r#"utouch -m -r fixture.json d e"#,
                result: None,
            },
            Example {
                description: r#"Changes the last accessed time of "fixture.json" to a date"#,
                example: r#"utouch -a -d "August 24, 2019; 12:30:30" fixture.json"#,
                result: None,
            },
        ]
    }
}
