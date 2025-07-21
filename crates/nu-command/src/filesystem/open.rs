#[allow(deprecated)]
use nu_engine::{command_prelude::*, current_dir, eval_call};
use nu_protocol::{
    DataSource, NuGlob, PipelineMetadata, ast,
    debugger::{WithDebug, WithoutDebug},
    shell_error::{self, io::IoError},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

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

    fn description(&self) -> &str {
        "Load a file into a cell, converting to table if possible (avoid by appending '--raw')."
    }

    fn extra_description(&self) -> &str {
        "Support to automatically parse files with an extension `.xyz` can be provided by a `from xyz` command in scope."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "load",
            "read",
            "load_file",
            "read_file",
            "cat",
            "get-content",
        ]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("open")
            .input_output_types(vec![
                (Type::Nothing, Type::Any),
                (Type::String, Type::Any),
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                (Type::Any, Type::Any),
            ])
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
        #[allow(deprecated)]
        let cwd = current_dir(engine_state, stack)?;
        let mut paths = call.rest::<Spanned<NuGlob>>(engine_state, stack, 0)?;

        if paths.is_empty() && !call.has_positional_args(stack, 0) {
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

            for path in
                nu_engine::glob_from(&path, &cwd, call_span, None, engine_state.signals().clone())
                    .map_err(|err| match err {
                        ShellError::Io(mut err) => {
                            err.kind = err.kind.not_found_as(NotFound::File);
                            err.span = arg_span;
                            err.into()
                        }
                        _ => err,
                    })?
                    .1
            {
                let path = path?;
                let path = Path::new(&path);

                if permission_denied(path) {
                    let err = IoError::new(
                        shell_error::io::ErrorKind::from_std(std::io::ErrorKind::PermissionDenied),
                        arg_span,
                        PathBuf::from(path),
                    );

                    #[cfg(unix)]
                    let err = {
                        let mut err = err;
                        err.additional_context = Some(
                            match path.metadata() {
                                Ok(md) => format!(
                                    "The permissions of {:o} does not allow access for this user",
                                    md.permissions().mode() & 0o0777
                                ),
                                Err(e) => e.to_string(),
                            }
                            .into(),
                        );
                        err
                    };

                    return Err(err.into());
                } else {
                    #[cfg(feature = "sqlite")]
                    if !raw {
                        let res = SQLiteDatabase::try_from_path(
                            path,
                            arg_span,
                            engine_state.signals().clone(),
                        )
                        .map(|db| db.into_value(call.head).into_pipeline_data());

                        if res.is_ok() {
                            return res;
                        }
                    }

                    if path.is_dir() {
                        // At least under windows this check ensures that we don't get a
                        // permission denied error on directories
                        return Err(ShellError::Io(IoError::new(
                            #[allow(
                                deprecated,
                                reason = "we don't have a IsADirectory variant here, so we provide one"
                            )]
                            shell_error::io::ErrorKind::from_std(std::io::ErrorKind::IsADirectory),
                            arg_span,
                            PathBuf::from(path),
                        )));
                    }

                    let file = std::fs::File::open(path)
                        .map_err(|err| IoError::new(err, arg_span, PathBuf::from(path)))?;

                    // No content_type by default - Is added later if no converter is found
                    let stream = PipelineData::byte_stream(
                        ByteStream::file(file, call_span, engine_state.signals().clone()),
                        Some(PipelineMetadata {
                            data_source: DataSource::FilePath(path.to_path_buf()),
                            content_type: None,
                        }),
                    );

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
                                .find_decl(format!("from {ext}").as_bytes(), &[])
                                .map(|id| (id, ext.to_string()))
                        })
                    });

                    match converter {
                        Some((converter_id, ext)) => {
                            let open_call = ast::Call {
                                decl_id: converter_id,
                                head: call_span,
                                arguments: vec![],
                                parser_info: HashMap::new(),
                            };
                            let command_output = if engine_state.is_debugging() {
                                eval_call::<WithDebug>(engine_state, stack, &open_call, stream)
                            } else {
                                eval_call::<WithoutDebug>(engine_state, stack, &open_call, stream)
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
                        None => {
                            // If no converter was found, add content-type metadata
                            let content_type = path
                                .extension()
                                .map(|ext| ext.to_string_lossy().to_string())
                                .and_then(|ref s| detect_content_type(s));

                            let stream_with_content_type =
                                stream.set_metadata(Some(PipelineMetadata {
                                    data_source: DataSource::FilePath(path.to_path_buf()),
                                    content_type,
                                }));
                            output.push(stream_with_content_type);
                        }
                    }
                }
            }
        }

        if output.is_empty() {
            Ok(PipelineData::empty())
        } else if output.len() == 1 {
            Ok(output.remove(0))
        } else {
            Ok(output
                .into_iter()
                .flatten()
                .into_pipeline_data(call_span, engine_state.signals().clone()))
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
            Example {
                description: "Show the extensions for which the `open` command will automatically parse",
                example: r#"scope commands
    | where name starts-with "from "
    | insert extension { get name | str replace -r "^from " "" | $"*.($in)" }
    | select extension name
    | rename extension command
"#,
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
            current_extension = format!("{part}.{current_extension}");
        }
        extensions.push(current_extension.clone());
    }

    extensions.pop();
    extensions.reverse();

    extensions
}

fn detect_content_type(extension: &str) -> Option<String> {
    // This will allow the overriding of metadata to be consistent with
    // the content type
    match extension {
        // Per RFC-9512, application/yaml should be used
        "yaml" | "yml" => Some("application/yaml".to_string()),
        "nu" => Some("application/x-nuscript".to_string()),
        "json" | "jsonl" | "ndjson" => Some("application/json".to_string()),
        "nuon" => Some("application/x-nuon".to_string()),
        _ => mime_guess::from_ext(extension)
            .first()
            .map(|mime| mime.to_string()),
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_content_type() {}
}
