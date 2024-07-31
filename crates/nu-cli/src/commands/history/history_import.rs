use nu_engine::command_prelude::*;
use nu_protocol::HistoryFileFormat;

use reedline::{
    FileBackedHistory, History, HistoryItem, HistoryItemId, ReedlineError, SearchQuery,
    SqliteBackedHistory,
};

use crate::{
    commands::history::fields,
    config_files::{HISTORY_FILE_SQLITE, HISTORY_FILE_TXT},
};

#[derive(Clone)]
pub struct HistoryImport;

impl Command for HistoryImport {
    fn name(&self) -> &str {
        "history import"
    }

    fn usage(&self) -> &str {
        "Import command line history"
    }

    fn extra_usage(&self) -> &str {
        r#"Can import history from input, either successive command lines or more detailed records. If providing records, available fields are:
    command_line, id, start_timestamp, hostname, cwd, duration, exit_status.

If no input is provided, will import all history items from existing history in the other format: if current history is stored in sqlite, it will store it in plain text and vice versa."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history import")
            .category(Category::History)
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::List(Box::new(Type::String)), Type::Nothing),
                (Type::table(), Type::Nothing),
            ])
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "history import",
                description:
                    "Append all items from history in the other format to the current history",
                result: None,
            },
            Example {
                example: "echo foo | history import",
                description: "Append `foo` to the current history",
                result: None,
            },
            Example {
                example: "[[ command_line cwd ]; [ foo /home ]] | history import",
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
        let ok = Ok(Value::nothing(call.head).into_pipeline_data());

        let Some(history) = engine_state.history_config() else {
            return ok;
        };
        let Some(config_path) = nu_path::nu_config_dir() else {
            return Err(ShellError::ConfigDirNotFound {
                span: Some(call.head),
            });
        };

        let new_sqlite_backend = || {
            SqliteBackedHistory::with_file(config_path.join(HISTORY_FILE_SQLITE), None, None)
                .map_err(error_from_reedline)
        };

        let new_file_backend = || {
            FileBackedHistory::with_file(
                history.max_size as usize,
                config_path.join(HISTORY_FILE_TXT),
            )
            .map_err(error_from_reedline)
        };

        match input {
            PipelineData::Empty => {
                let mut sqlite = new_sqlite_backend()?;
                let mut plaintext = new_file_backend()?;
                let (src, dst): (&dyn History, &mut dyn History) = match history.file_format {
                    HistoryFileFormat::Sqlite => (&plaintext, &mut sqlite),
                    HistoryFileFormat::PlainText => (&sqlite, &mut plaintext),
                };

                let items = src
                    .search(SearchQuery::everything(
                        reedline::SearchDirection::Forward,
                        None,
                    ))
                    .map_err(error_from_reedline)?
                    .into_iter()
                    .map(Ok);
                import(dst, items)
            }
            _ => {
                let input = input.into_iter().map(item_from_value);
                match history.file_format {
                    HistoryFileFormat::Sqlite => import(&mut new_sqlite_backend()?, input),
                    HistoryFileFormat::PlainText => import(&mut new_file_backend()?, input),
                }
            }
        }?;

        ok
    }
}

fn import(
    dst: &mut dyn History,
    src: impl Iterator<Item = Result<HistoryItem, ShellError>>,
) -> Result<(), ShellError> {
    for item in src {
        dst.save(item?).map_err(error_from_reedline)?;
    }
    Ok(())
}

fn error_from_reedline(e: ReedlineError) -> ShellError {
    // TODO: Should we add a new ShellError variant?
    ShellError::IOError {
        msg: format!("reedline error: {e}"),
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
            return Err(ShellError::NotFound {
                span: Span::unknown(),
            })
        }
    };

    fn get<T>(
        rec: &mut Record,
        field: &'static str,
        f: impl FnOnce(Value) -> Result<T, ShellError>,
    ) -> Result<Option<T>, ShellError> {
        rec.remove(field).map(f).transpose()
    }

    let item = HistoryItem {
        command_line: cmd,
        id: get(&mut rec, fields::ID, |v| {
            Ok(HistoryItemId::new(v.as_int()?))
        })?,
        start_timestamp: get(&mut rec, fields::START_TIMESTAMP, |v| {
            Ok(v.as_date()?.to_utc())
        })?,
        hostname: get(&mut rec, fields::HOSTNAME, |v| Ok(v.as_str()?.to_owned()))?,
        cwd: get(&mut rec, fields::CWD, |v| Ok(v.as_str()?.to_owned()))?,
        exit_status: get(&mut rec, fields::EXIT_STATUS, |v| v.as_i64())?,
        duration: get(&mut rec, fields::DURATION, duration_from_value)?,
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

fn duration_from_value(v: Value) -> Result<std::time::Duration, ShellError> {
    chrono::Duration::nanoseconds(v.as_duration()?)
        .to_std()
        .map_err(|_| ShellError::IOError {
            msg: "negative duration not supported".to_string(),
        })
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

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
            ("command_line", Value::string("foo", span)),
            ("id", Value::int(1, span)),
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
                id: Some(HistoryItemId::new(1)),
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
}
