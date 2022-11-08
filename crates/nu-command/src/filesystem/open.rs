use crate::filesystem::util::BufferedReader;
use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, RawStream, ShellError, Signature, Spanned, SyntaxShape, Value,
};
use std::io::BufReader;

#[cfg(feature = "database")]
use crate::database::SQLiteDatabase;

#[cfg(feature = "database")]
use nu_protocol::IntoPipelineData;

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

    fn search_terms(&self) -> Vec<&str> {
        vec!["load", "read", "load_file", "read_file"]
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

        let path = {
            if let Some(path_val) = path {
                Some(Spanned {
                    item: nu_utils::strip_ansi_string_unlikely(path_val.item),
                    span: path_val.span,
                })
            } else {
                path
            }
        };

        let path = if let Some(path) = path {
            path
        } else {
            // Collect a filename from the input
            match input {
                PipelineData::Value(Value::Nothing { .. }, ..) => {
                    return Err(ShellError::MissingParameter(
                        "needs filename".to_string(),
                        call.head,
                    ))
                }
                PipelineData::Value(val, ..) => val.as_spanned_string()?,
                _ => {
                    return Err(ShellError::MissingParameter(
                        "needs filename".to_string(),
                        call.head,
                    ));
                }
            }
        };
        let arg_span = path.span;
        let path_no_whitespace = &path.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));
        let path = Path::new(path_no_whitespace);

        if permission_denied(path) {
            #[cfg(unix)]
            let error_msg = match path.metadata() {
                Ok(md) => format!(
                    "The permissions of {:o} does not allow access for this user",
                    md.permissions().mode() & 0o0777
                ),
                Err(e) => e.to_string(),
            };

            #[cfg(not(unix))]
            let error_msg = String::from("Permission denied");
            Err(ShellError::GenericError(
                "Permission denied".into(),
                error_msg,
                Some(arg_span),
                None,
                Vec::new(),
            ))
        } else {
            #[cfg(feature = "database")]
            if !raw {
                let res = SQLiteDatabase::try_from_path(path, arg_span)
                    .map(|db| db.into_value(call.head).into_pipeline_data());

                if res.is_ok() {
                    return res;
                }
            }

            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(err) => {
                    return Err(ShellError::GenericError(
                        "Permission denied".into(),
                        err.to_string(),
                        Some(arg_span),
                        None,
                        Vec::new(),
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
                match engine_state.find_decl(format!("from {}", ext).as_bytes(), &[]) {
                    Some(converter_id) => {
                        let decl = engine_state.get_decl(converter_id);
                        if let Some(block_id) = decl.get_block_id() {
                            let block = engine_state.get_block(block_id);
                            eval_block(engine_state, stack, block, output, false, false)
                        } else {
                            decl.run(engine_state, stack, &Call::new(call_span), output)
                        }
                        .map_err(|inner| {
                            ShellError::GenericError(
                                format!("Error while parsing as {}", ext),
                                format!("Could not parse '{}' with `from {}`", path.display(), ext),
                                Some(arg_span),
                                Some(format!("Check out `help from {}` or `help from` for more options or open raw data with `open --raw '{}'`", ext, path.display())),
                                vec![inner],
                            )
                        })
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

fn permission_denied(dir: impl AsRef<Path>) -> bool {
    match dir.as_ref().read_dir() {
        Err(e) => matches!(e.kind(), std::io::ErrorKind::PermissionDenied),
        Ok(_) => false,
    }
}
