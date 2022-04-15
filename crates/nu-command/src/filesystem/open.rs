use nu_engine::{eval_block, get_full_help, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, RawStream, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value,
};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, Row};
use std::io::{BufRead, BufReader, Read, Seek};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Clone)]
pub struct Open;

impl Command for Open {
    fn name(&self) -> &str {
        "open"
    }

    fn usage(&self) -> &str {
        "Load a file into a cell, converting to table if possible (avoid by appending '--raw')."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("open")
            .optional("filename", SyntaxShape::Filepath, "the filename to use")
            .switch("raw", "open file as raw binary", Some('r'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let raw = call.has_flag("raw");

        let call_span = call.head;
        let ctrlc = engine_state.ctrlc.clone();

        let path = call.opt::<Spanned<String>>(engine_state, stack, 0)?;

        let path = if let Some(path) = path {
            path
        } else {
            // Collect a filename from the input
            match input {
                PipelineData::Value(Value::Nothing { .. }, ..) => {
                    return Ok(Value::String {
                        val: get_full_help(
                            &Open.signature(),
                            &Open.examples(),
                            engine_state,
                            stack,
                        ),
                        span: call.head,
                    }
                    .into_pipeline_data())
                }
                PipelineData::Value(val, ..) => val.as_spanned_string()?,
                _ => {
                    return Ok(Value::String {
                        val: get_full_help(
                            &Open.signature(),
                            &Open.examples(),
                            engine_state,
                            stack,
                        ),
                        span: call.head,
                    }
                    .into_pipeline_data())
                }
            }
        };
        let arg_span = path.span;
        let path = Path::new(&path.item);

        if permission_denied(&path) {
            #[cfg(unix)]
            let error_msg = format!(
                "The permissions of {:o} do not allow access for this user",
                path.metadata()
                    .expect("this shouldn't be called since we already know there is a dir")
                    .permissions()
                    .mode()
                    & 0o0777
            );
            #[cfg(not(unix))]
            let error_msg = String::from("Permission denied");
            Err(ShellError::SpannedLabeledError(
                "Permission denied".into(),
                error_msg,
                arg_span,
            ))
        } else {
            let mut file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(err) => {
                    return Err(ShellError::SpannedLabeledError(
                        "Permission denied".into(),
                        err.to_string(),
                        arg_span,
                    ));
                }
            };

            // Peek at the file to see if we can detect a SQLite database
            if !raw {
                let sqlite_magic_bytes = "SQLite format 3\0".as_bytes();
                let mut buf: [u8; 16] = [0; 16];

                if file.read_exact(&mut buf).is_ok() && buf == sqlite_magic_bytes {
                    return open_and_read_sqlite_db(path, call_span)
                        .map(|val| PipelineData::Value(val, None));
                }

                if file.rewind().is_err() {
                    return Err(ShellError::IOError("Failed to rewind file".into()));
                };
            }

            let buf_reader = BufReader::new(file);

            let output = PipelineData::ExternalStream {
                stdout: Some(RawStream::new(
                    Box::new(BufferedReader { input: buf_reader }),
                    ctrlc,
                    call_span,
                )),
                stderr: None,
                exit_code: None,
                span: call_span,
                metadata: None,
            };

            let ext = if raw {
                None
            } else {
                path.extension()
                    .map(|name| name.to_string_lossy().to_string())
            };

            if let Some(ext) = ext {
                match engine_state.find_decl(format!("from {}", ext).as_bytes()) {
                    Some(converter_id) => {
                        let decl = engine_state.get_decl(converter_id);
                        if let Some(block_id) = decl.get_block_id() {
                            let block = engine_state.get_block(block_id);
                            eval_block(engine_state, stack, block, output, false, false)
                        } else {
                            decl.run(engine_state, stack, &Call::new(arg_span), output)
                        }
                    }
                    None => Ok(output),
                }
            } else {
                Ok(output)
            }
        }
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "Open a file, with structure (based on file extension or SQLite database header)",
                example: "open myfile.json",
                result: None,
            },
            Example {
                description: "Open a file, as raw bytes",
                example: "open myfile.json --raw",
                result: None,
            },
            Example {
                description: "Open a file, using the input to get filename",
                example: "echo 'myfile.txt' | open",
                result: None,
            },
            Example {
                description: "Open a file, and decode it by the specified encoding",
                example: "open myfile.txt --raw | decode utf-8",
                result: None,
            },
        ]
    }
}

fn open_and_read_sqlite_db(path: &Path, call_span: Span) -> Result<Value, nu_protocol::ShellError> {
    let path = path.to_string_lossy().to_string();

    match Connection::open(path) {
        Ok(conn) => match read_sqlite_db(conn, call_span) {
            Ok(data) => Ok(data),
            Err(err) => Err(ShellError::SpannedLabeledError(
                "Failed to read from SQLite database".into(),
                err.to_string(),
                call_span,
            )),
        },
        Err(err) => Err(ShellError::SpannedLabeledError(
            "Failed to open SQLite database".into(),
            err.to_string(),
            call_span,
        )),
    }
}

fn read_sqlite_db(conn: Connection, call_span: Span) -> Result<Value, rusqlite::Error> {
    let mut table_names: Vec<String> = Vec::new();
    let mut tables: Vec<Value> = Vec::new();

    let mut get_table_names =
        conn.prepare("SELECT name from sqlite_master where type = 'table'")?;
    let rows = get_table_names.query_map([], |row| row.get(0))?;

    for row in rows {
        let table_name: String = row?;
        table_names.push(table_name.clone());

        let mut rows = Vec::new();
        let mut table_stmt = conn.prepare(&format!("select * from [{}]", table_name))?;
        let mut table_rows = table_stmt.query([])?;
        while let Some(table_row) = table_rows.next()? {
            rows.push(convert_sqlite_row_to_nu_value(table_row, call_span))
        }

        let table_record = Value::List {
            vals: rows,
            span: call_span,
        };

        tables.push(table_record);
    }

    Ok(Value::Record {
        cols: table_names,
        vals: tables,
        span: call_span,
    })
}

fn convert_sqlite_row_to_nu_value(row: &Row, span: Span) -> Value {
    let mut vals = Vec::new();
    let colnamestr = row.as_ref().column_names().to_vec();
    let colnames = colnamestr.iter().map(|s| s.to_string()).collect();

    for (i, c) in row.as_ref().column_names().iter().enumerate() {
        let _column = c.to_string();
        let val = convert_sqlite_value_to_nu_value(row.get_ref_unwrap(i), span);
        vals.push(val);
    }

    Value::Record {
        cols: colnames,
        vals,
        span,
    }
}

fn convert_sqlite_value_to_nu_value(value: ValueRef, span: Span) -> Value {
    match value {
        ValueRef::Null => Value::Nothing { span },
        ValueRef::Integer(i) => Value::Int { val: i, span },
        ValueRef::Real(f) => Value::Float { val: f, span },
        ValueRef::Text(buf) => {
            let s = match std::str::from_utf8(buf) {
                Ok(v) => v,
                Err(_) => {
                    return Value::Error {
                        error: ShellError::NonUtf8(span),
                    }
                }
            };
            Value::String {
                val: s.to_string(),
                span,
            }
        }
        ValueRef::Blob(u) => Value::Binary {
            val: u.to_vec(),
            span,
        },
    }
}

fn permission_denied(dir: impl AsRef<Path>) -> bool {
    match dir.as_ref().read_dir() {
        Err(e) => matches!(e.kind(), std::io::ErrorKind::PermissionDenied),
        Ok(_) => false,
    }
}

pub struct BufferedReader<R: Read> {
    input: BufReader<R>,
}

impl<R: Read> BufferedReader<R> {
    pub fn new(input: BufReader<R>) -> Self {
        Self { input }
    }
}

impl<R: Read> Iterator for BufferedReader<R> {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let buffer = self.input.fill_buf();
        match buffer {
            Ok(s) => {
                let result = s.to_vec();

                let buffer_len = s.len();

                if buffer_len == 0 {
                    None
                } else {
                    self.input.consume(buffer_len);

                    Some(Ok(result))
                }
            }
            Err(e) => Some(Err(ShellError::IOError(e.to_string()))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_read_empty_db() {
        let db = Connection::open_in_memory().unwrap();
        let converted_db = read_sqlite_db(db, Span::test_data()).unwrap();

        let expected = Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span::test_data(),
        };

        assert_eq!(converted_db, expected);
    }

    #[test]
    fn can_read_empty_table() {
        let db = Connection::open_in_memory().unwrap();

        db.execute(
            "CREATE TABLE person (
                    id     INTEGER PRIMARY KEY,
                    name   TEXT NOT NULL,
                    data   BLOB
                    )",
            [],
        )
        .unwrap();
        let converted_db = read_sqlite_db(db, Span::test_data()).unwrap();

        let expected = Value::Record {
            cols: vec!["person".to_string()],
            vals: vec![Value::List {
                vals: vec![],
                span: Span::test_data(),
            }],
            span: Span::test_data(),
        };

        assert_eq!(converted_db, expected);
    }

    #[test]
    fn can_read_null_and_non_null_data() {
        let span = Span::test_data();
        let db = Connection::open_in_memory().unwrap();

        db.execute(
            "CREATE TABLE item (
                    id     INTEGER PRIMARY KEY,
                    name   TEXT
                    )",
            [],
        )
        .unwrap();

        db.execute("INSERT INTO item (id, name) VALUES (123, NULL)", [])
            .unwrap();

        db.execute("INSERT INTO item (id, name) VALUES (456, 'foo bar')", [])
            .unwrap();

        let converted_db = read_sqlite_db(db, span).unwrap();

        let expected = Value::Record {
            cols: vec!["item".to_string()],
            vals: vec![Value::List {
                vals: vec![
                    Value::Record {
                        cols: vec!["id".to_string(), "name".to_string()],
                        vals: vec![
                            Value::Int {
                                val: 123,
                                span: span,
                            },
                            Value::Nothing { span: span },
                        ],
                        span: span,
                    },
                    Value::Record {
                        cols: vec!["id".to_string(), "name".to_string()],
                        vals: vec![
                            Value::Int {
                                val: 456,
                                span: span,
                            },
                            Value::String {
                                val: "foo bar".to_string(),
                                span: span,
                            },
                        ],
                        span: span,
                    },
                ],
                span: span,
            }],
            span,
        };

        assert_eq!(converted_db, expected);
    }
}
