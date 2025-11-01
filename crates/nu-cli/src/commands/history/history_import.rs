use std::path::{Path, PathBuf};

use nu_engine::command_prelude::*;
use nu_protocol::{
    HistoryFileFormat,
    shell_error::{self, io::IoError},
};

use reedline::{
    FileBackedHistory, History, HistoryItem, ReedlineError, SearchQuery, SqliteBackedHistory,
};

use super::fields;

#[derive(Clone)]
pub struct HistoryImport;

impl Command for HistoryImport {
    fn name(&self) -> &str {
        "history import"
    }

    fn description(&self) -> &str {
        "Import command line history."
    }

    fn extra_description(&self) -> &str {
        r#"Can import history from input, either successive command lines or more detailed records. If providing records, available fields are:
    command, start_timestamp, hostname, cwd, duration, exit_status.

If no input is provided, will import all history items from existing history in the other format: if current history is stored in sqlite, it will store it in plain text and vice versa.

Note that history item IDs are ignored when importing from file."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history import")
            .category(Category::History)
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::String, Type::Nothing),
                (Type::List(Box::new(Type::String)), Type::Nothing),
                (Type::table(), Type::Nothing),
            ])
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "history import",
                description: "Append all items from history in the other format to the current history",
                result: None,
            },
            Example {
                example: "echo foo | history import",
                description: "Append `foo` to the current history",
                result: None,
            },
            Example {
                example: "[[ command cwd ]; [ foo /home ]] | history import",
                description: "Append `foo` ran from `/home` to the current history",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let ok = Ok(Value::nothing(call.head).into_pipeline_data());

        let Some(history) = engine_state.history_config() else {
            return ok;
        };
        let Some(current_history_path) = history.file_path() else {
            return Err(ShellError::ConfigDirNotFound { span });
        };
        if let Some(bak_path) = backup(&current_history_path, span)? {
            println!("Backed history to {}", bak_path.display());
        }
        match input {
            PipelineData::Empty => {
                let other_format = match history.file_format {
                    HistoryFileFormat::Sqlite => HistoryFileFormat::Plaintext,
                    HistoryFileFormat::Plaintext => HistoryFileFormat::Sqlite,
                };
                let src = new_backend(other_format, None, call.head)?;
                let mut dst =
                    new_backend(history.file_format, Some(current_history_path), call.head)?;
                let items = src
                    .search(SearchQuery::everything(
                        reedline::SearchDirection::Forward,
                        None,
                    ))
                    .map_err(error_from_reedline)?
                    .into_iter()
                    .map(Ok);
                import(dst.as_mut(), items)
            }
            _ => {
                let input = input.into_iter().map(item_from_value);
                import(
                    new_backend(history.file_format, Some(current_history_path), call.head)?
                        .as_mut(),
                    input,
                )
            }
        }?;

        ok
    }
}

fn new_backend(
    format: HistoryFileFormat,
    path: Option<PathBuf>,
    span: Span,
) -> Result<Box<dyn History>, ShellError> {
    let path = match path {
        Some(path) => path,
        None => {
            let Some(mut path) = nu_path::nu_config_dir() else {
                return Err(ShellError::ConfigDirNotFound { span });
            };
            path.push(format.default_file_name());
            path.into_std_path_buf()
        }
    };

    fn map(
        result: Result<impl History + 'static, ReedlineError>,
    ) -> Result<Box<dyn History>, ShellError> {
        result
            .map(|x| Box::new(x) as Box<dyn History>)
            .map_err(error_from_reedline)
    }
    match format {
        // Use a reasonably large value for maximum capacity.
        HistoryFileFormat::Plaintext => map(FileBackedHistory::with_file(0xfffffff, path)),
        HistoryFileFormat::Sqlite => map(SqliteBackedHistory::with_file(path, None, None)),
    }
}

fn import(
    dst: &mut dyn History,
    src: impl Iterator<Item = Result<HistoryItem, ShellError>>,
) -> Result<(), ShellError> {
    for item in src {
        let mut item = item?;
        item.id = None;
        dst.save(item).map_err(error_from_reedline)?;
    }
    Ok(())
}

fn error_from_reedline(e: ReedlineError) -> ShellError {
    // TODO: Should we add a new ShellError variant?
    ShellError::GenericError {
        error: "Reedline error".to_owned(),
        msg: format!("{e}"),
        span: None,
        help: None,
        inner: Vec::new(),
    }
}

fn item_from_value(v: Value) -> Result<HistoryItem, ShellError> {
    let span = v.span();
    match v {
        Value::Record { val, .. } => item_from_record(val.into_owned(), span),
        Value::String { val, .. } => Ok(HistoryItem {
            command_line: val,
            id: None,
            start_timestamp: None,
            session_id: None,
            hostname: None,
            cwd: None,
            duration: None,
            exit_status: None,
            more_info: None,
        }),
        _ => Err(ShellError::UnsupportedInput {
            msg: "Only list and record inputs are supported".to_owned(),
            input: v.get_type().to_string(),
            msg_span: span,
            input_span: span,
        }),
    }
}

fn item_from_record(mut rec: Record, span: Span) -> Result<HistoryItem, ShellError> {
    let cmd = match rec.remove(fields::COMMAND_LINE) {
        Some(v) => v.as_str()?.to_owned(),
        None => {
            return Err(ShellError::TypeMismatch {
                err_message: format!("missing column: {}", fields::COMMAND_LINE),
                span,
            });
        }
    };

    fn get<T>(
        rec: &mut Record,
        field: &'static str,
        f: impl FnOnce(Value) -> Result<T, ShellError>,
    ) -> Result<Option<T>, ShellError> {
        rec.remove(field).map(f).transpose()
    }

    let rec = &mut rec;
    let item = HistoryItem {
        command_line: cmd,
        id: None,
        start_timestamp: get(rec, fields::START_TIMESTAMP, |v| Ok(v.as_date()?.to_utc()))?,
        hostname: get(rec, fields::HOSTNAME, |v| Ok(v.as_str()?.to_owned()))?,
        cwd: get(rec, fields::CWD, |v| Ok(v.as_str()?.to_owned()))?,
        exit_status: get(rec, fields::EXIT_STATUS, |v| v.as_int())?,
        duration: get(rec, fields::DURATION, |v| duration_from_value(v, span))?,
        more_info: None,
        // TODO: Currently reedline doesn't let you create session IDs.
        session_id: None,
    };

    if !rec.is_empty() {
        let cols = rec.columns().map(|s| s.as_str()).collect::<Vec<_>>();
        return Err(ShellError::TypeMismatch {
            err_message: format!("unsupported column names: {}", cols.join(", ")),
            span,
        });
    }
    Ok(item)
}

fn duration_from_value(v: Value, span: Span) -> Result<std::time::Duration, ShellError> {
    chrono::Duration::nanoseconds(v.as_duration()?)
        .to_std()
        .map_err(|_| ShellError::NeedsPositiveValue { span })
}

fn find_backup_path(path: &Path, span: Span) -> Result<PathBuf, ShellError> {
    let Ok(mut bak_path) = path.to_path_buf().into_os_string().into_string() else {
        // This isn't fundamentally problem, but trying to work with OsString is a nightmare.
        return Err(ShellError::GenericError {
            error: "History path not UTF-8".to_string(),
            msg: "History path must be representable as UTF-8".to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        });
    };
    bak_path.push_str(".bak");
    if !Path::new(&bak_path).exists() {
        return Ok(bak_path.into());
    }
    let base_len = bak_path.len();
    for i in 1..100 {
        use std::fmt::Write;
        bak_path.truncate(base_len);
        write!(&mut bak_path, ".{i}").ok();
        if !Path::new(&bak_path).exists() {
            return Ok(PathBuf::from(bak_path));
        }
    }
    Err(ShellError::GenericError {
        error: "Too many backup files".to_string(),
        msg: "Found too many existing backup files".to_string(),
        span: Some(span),
        help: None,
        inner: vec![],
    })
}

fn backup(path: &Path, span: Span) -> Result<Option<PathBuf>, ShellError> {
    match path.metadata() {
        Ok(md) if md.is_file() => (),
        Ok(_) => {
            return Err(IoError::new_with_additional_context(
                shell_error::io::ErrorKind::NotAFile,
                span,
                PathBuf::from(path),
                "history path exists but is not a file",
            )
            .into());
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(IoError::new_internal(
                e,
                "Could not get metadata",
                nu_protocol::location!(),
            )
            .into());
        }
    }
    let bak_path = find_backup_path(path, span)?;
    std::fs::copy(path, &bak_path).map_err(|err| {
        IoError::new_internal(
            err.not_found_as(NotFound::File),
            "Could not copy backup",
            nu_protocol::location!(),
        )
    })?;
    Ok(Some(bak_path))
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_item_from_value_string() -> Result<(), ShellError> {
        let item = item_from_value(Value::string("foo", Span::unknown()))?;
        assert_eq!(
            item,
            HistoryItem {
                command_line: "foo".to_string(),
                id: None,
                start_timestamp: None,
                session_id: None,
                hostname: None,
                cwd: None,
                duration: None,
                exit_status: None,
                more_info: None
            }
        );
        Ok(())
    }

    #[test]
    fn test_item_from_value_record() {
        let span = Span::unknown();
        let rec = new_record(&[
            ("command", Value::string("foo", span)),
            (
                "start_timestamp",
                Value::date(
                    DateTime::parse_from_rfc3339("1996-12-19T16:39:57-08:00").unwrap(),
                    span,
                ),
            ),
            ("hostname", Value::string("localhost", span)),
            ("cwd", Value::string("/home/test", span)),
            ("duration", Value::duration(100_000_000, span)),
            ("exit_status", Value::int(42, span)),
        ]);
        let item = item_from_value(rec).unwrap();
        assert_eq!(
            item,
            HistoryItem {
                command_line: "foo".to_string(),
                id: None,
                start_timestamp: Some(
                    DateTime::parse_from_rfc3339("1996-12-19T16:39:57-08:00")
                        .unwrap()
                        .to_utc()
                ),
                hostname: Some("localhost".to_string()),
                cwd: Some("/home/test".to_string()),
                duration: Some(std::time::Duration::from_nanos(100_000_000)),
                exit_status: Some(42),

                session_id: None,
                more_info: None
            }
        );
    }

    #[test]
    fn test_item_from_value_record_extra_field() {
        let span = Span::unknown();
        let rec = new_record(&[
            ("command_line", Value::string("foo", span)),
            ("id_nonexistent", Value::int(1, span)),
        ]);
        assert!(item_from_value(rec).is_err());
    }

    #[test]
    fn test_item_from_value_record_bad_type() {
        let span = Span::unknown();
        let rec = new_record(&[
            ("command_line", Value::string("foo", span)),
            ("id", Value::string("one".to_string(), span)),
        ]);
        assert!(item_from_value(rec).is_err());
    }

    fn new_record(rec: &[(&'static str, Value)]) -> Value {
        let span = Span::unknown();
        let rec = Record::from_raw_cols_vals(
            rec.iter().map(|(col, _)| col.to_string()).collect(),
            rec.iter().map(|(_, val)| val.clone()).collect(),
            span,
            span,
        )
        .unwrap();
        Value::record(rec, span)
    }

    #[rstest]
    #[case::no_backup(&["history.dat"], "history.dat.bak")]
    #[case::backup_exists(&["history.dat", "history.dat.bak"], "history.dat.bak.1")]
    #[case::multiple_backups_exists( &["history.dat", "history.dat.bak", "history.dat.bak.1"], "history.dat.bak.2")]
    fn test_find_backup_path(#[case] existing: &[&str], #[case] want: &str) {
        let dir = tempfile::tempdir().unwrap();
        for name in existing {
            std::fs::File::create_new(dir.path().join(name)).unwrap();
        }
        let got = find_backup_path(&dir.path().join("history.dat"), Span::test_data()).unwrap();
        assert_eq!(got, dir.path().join(want))
    }

    #[test]
    fn test_backup() {
        let dir = tempfile::tempdir().unwrap();
        let mut history = std::fs::File::create_new(dir.path().join("history.dat")).unwrap();
        use std::io::Write;
        write!(&mut history, "123").unwrap();
        let want_bak_path = dir.path().join("history.dat.bak");
        assert_eq!(
            backup(&dir.path().join("history.dat"), Span::test_data()),
            Ok(Some(want_bak_path.clone()))
        );
        let got_data = String::from_utf8(std::fs::read(want_bak_path).unwrap()).unwrap();
        assert_eq!(got_data, "123");
    }

    #[test]
    fn test_backup_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let bak_path = backup(&dir.path().join("history.dat"), Span::test_data()).unwrap();
        assert!(bak_path.is_none());
    }
}
