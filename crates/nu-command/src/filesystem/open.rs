use super::util::get_rest_for_glob_pattern;
use nu_engine::{command_prelude::*, current_dir, get_eval_block};
use nu_protocol::{BufferedReader, DataSource, NuGlob, PipelineMetadata, RawStream};
use std::{io::BufReader, path::Path};

#[cfg(feature = "sqlite")]
use crate::database::SQLiteDatabase;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone)]
pub struct Open;

impl Command for Open {
    fn name(&self) -> &str {
        "open"
    }

    fn usage(&self) -> &str {
        "Load a file into a cell, converting to table if possible (avoid by appending '--raw')."
    }

    fn extra_usage(&self) -> &str {
        "Support to automatically parse files with an extension `.xyz` can be provided by a `from xyz` command in scope."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["load", "read", "load_file", "read_file"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("open")
            .input_output_types(vec![(Type::Nothing, Type::Any), (Type::String, Type::Any)])
            .rest(
                "files",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::String]),
                "The file(s) to open.",
            )
            .switch("raw", "open file as raw binary", Some('r'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let raw = call.has_flag(engine_state, stack, "raw")?;
        let call_span = call.head;
        let ctrlc = engine_state.ctrlc.clone();
        let cwd = current_dir(engine_state, stack)?;
        let mut paths = get_rest_for_glob_pattern(engine_state, stack, call, 0)?;
        let eval_block = get_eval_block(engine_state);

        if paths.is_empty() && call.rest_iter(0).next().is_none() {
            // try to use path from pipeline input if there were no positional or spread args
            let (filename, span) = match input {
                PipelineData::Value(val, ..) => {
                    let span = val.span();
                    (val.coerce_into_string()?, span)
                }
                _ => {
                    return Err(ShellError::MissingParameter {
                        param_name: "needs filename".to_string(),
                        span: call.head,
                    });
                }
            };

            paths.push(Spanned {
                item: NuGlob::Expand(filename),
                span,
            });
        }

        let mut output = vec![];

        for mut path in paths {
            //FIXME: `open` should not have to do this
            path.item = path.item.strip_ansi_string_unlikely();

            let arg_span = path.span;
            // let path_no_whitespace = &path.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));

            for path in nu_engine::glob_from(&path, &cwd, call_span, None)
                .map_err(|err| match err {
                    ShellError::DirectoryNotFound { span, .. } => ShellError::FileNotFound {
                        file: path.item.to_string(),
                        span,
                    },
                    _ => err,
                })?
                .1
            {
                let path = path?;
                let path = Path::new(&path);

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
                    return Err(ShellError::GenericError {
                        error: "Permission denied".into(),
                        msg: error_msg,
                        span: Some(arg_span),
                        help: None,
                        inner: vec![],
                    });
                } else {
                    #[cfg(feature = "sqlite")]
                    if !raw {
                        let res = SQLiteDatabase::try_from_path(path, arg_span, ctrlc.clone())
                            .map(|db| db.into_value(call.head).into_pipeline_data());

                        if res.is_ok() {
                            return res;
                        }
                    }

                    let file = match std::fs::File::open(path) {
                        Ok(file) => file,
                        Err(err) => {
                            return Err(ShellError::GenericError {
                                error: "Permission denied".into(),
                                msg: err.to_string(),
                                span: Some(arg_span),
                                help: None,
                                inner: vec![],
                            });
                        }
                    };

                    let buf_reader = BufReader::new(file);

                    let file_contents = PipelineData::ExternalStream {
                        stdout: Some(RawStream::new(
                            Box::new(BufferedReader::new(buf_reader)),
                            ctrlc.clone(),
                            call_span,
                            None,
                        )),
                        stderr: None,
                        exit_code: None,
                        span: call_span,
                        metadata: Some(PipelineMetadata {
                            data_source: DataSource::FilePath(path.to_path_buf()),
                        }),
                        trim_end_newline: false,
                    };
                    let exts_opt: Option<Vec<String>> = if raw {
                        None
                    } else {
                        let path_str = path
                            .file_name()
                            .unwrap_or(std::ffi::OsStr::new(path))
                            .to_string_lossy()
                            .to_lowercase();
                        Some(extract_extensions(path_str.as_str()))
                    };

                    let converter = exts_opt.and_then(|exts| {
                        exts.iter().find_map(|ext| {
                            engine_state
                                .find_decl(format!("from {}", ext).as_bytes(), &[])
                                .map(|id| (id, ext.to_string()))
                        })
                    });

                    match converter {
                        Some((converter_id, ext)) => {
                            let decl = engine_state.get_decl(converter_id);
                            let command_output = if let Some(block_id) = decl.get_block_id() {
                                let block = engine_state.get_block(block_id);
                                eval_block(engine_state, stack, block, file_contents)
                            } else {
                                decl.run(engine_state, stack, &Call::new(call_span), file_contents)
                            };
                            output.push(command_output.map_err(|inner| {
                                    ShellError::GenericError{
                                        error: format!("Error while parsing as {ext}"),
                                        msg: format!("Could not parse '{}' with `from {}`", path.display(), ext),
                                        span: Some(arg_span),
                                        help: Some(format!("Check out `help from {}` or `help from` for more options or open raw data with `open --raw '{}'`", ext, path.display())),
                                        inner: vec![inner],
                                }
                                })?);
                        }
                        None => output.push(file_contents),
                    }
                }
            }
        }

        if output.is_empty() {
            Ok(PipelineData::Empty)
        } else if output.len() == 1 {
            Ok(output.remove(0))
        } else {
            Ok(output.into_iter().flatten().into_pipeline_data(ctrlc))
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
                example: "'myfile.txt' | open",
                result: None,
            },
            Example {
                description: "Open a file, and decode it by the specified encoding",
                example: "open myfile.txt --raw | decode utf-8",
                result: None,
            },
            Example {
                description: "Create a custom `from` parser to open newline-delimited JSON files with `open`",
                example: r#"def "from ndjson" [] { from json -o }; open myfile.ndjson"#,
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

fn extract_extensions(filename: &str) -> Vec<String> {
    let parts: Vec<&str> = filename.split('.').collect();
    let mut extensions: Vec<String> = Vec::new();
    let mut current_extension = String::new();

    for part in parts.iter().rev() {
        if current_extension.is_empty() {
            current_extension.push_str(part);
        } else {
            current_extension = format!("{}.{}", part, current_extension);
        }
        extensions.push(current_extension.clone());
    }

    extensions.pop();
    extensions.reverse();

    extensions
}
