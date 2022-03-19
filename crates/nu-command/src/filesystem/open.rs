use nu_engine::{get_full_help, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, RawStream, ShellError, Signature, Spanned,
    SyntaxShape, Value,
};
use std::io::{BufRead, BufReader, Read};

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
            Ok(PipelineData::Value(
                Value::Error {
                    error: ShellError::SpannedLabeledError(
                        "Permission denied".into(),
                        error_msg,
                        arg_span,
                    ),
                },
                None,
            ))
        } else {
            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(err) => {
                    return Ok(PipelineData::Value(
                        Value::Error {
                            error: ShellError::SpannedLabeledError(
                                "Permission denied".into(),
                                err.to_string(),
                                arg_span,
                            ),
                        },
                        None,
                    ));
                }
            };

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
                    Some(converter_id) => engine_state.get_decl(converter_id).run(
                        engine_state,
                        stack,
                        &Call::new(arg_span),
                        output,
                    ),
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
                description: "Open a file, with structure (based on file extension)",
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
