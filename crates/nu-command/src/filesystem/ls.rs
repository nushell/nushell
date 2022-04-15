use crate::DirBuilder;
use crate::DirInfo;
use chrono::{DateTime, Local};
use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, DataSource, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    PipelineMetadata, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};
use pathdiff::diff_paths;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct Ls;

impl Command for Ls {
    fn name(&self) -> &str {
        "ls"
    }

    fn usage(&self) -> &str {
        "List the files in a directory."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["dir"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("ls")
            // Using a string instead of a glob pattern shape so it won't auto-expand
            .optional("pattern", SyntaxShape::String, "the glob pattern to use")
            .switch("all", "Show hidden files", Some('a'))
            .switch(
                "long",
                "List all available columns for each entry",
                Some('l'),
            )
            .switch(
                "short-names",
                "Only print the file names and not the path",
                Some('s'),
            )
            .switch("full-paths", "display paths as absolute paths", Some('f'))
            .switch(
                "du",
                "Display the apparent directory size in place of the directory metadata size",
                Some('d'),
            )
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let all = call.has_flag("all");
        let long = call.has_flag("long");
        let short_names = call.has_flag("short-names");
        let full_paths = call.has_flag("full-paths");
        let du = call.has_flag("du");
        let ctrl_c = engine_state.ctrlc.clone();
        let call_span = call.head;
        let cwd = current_dir(engine_state, stack)?;

        let pattern_arg: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;

        let (path, p_tag, absolute_path) = match pattern_arg {
            Some(p) => {
                let p_tag = p.span;
                let mut p = PathBuf::from(p.item);

                let expanded = nu_path::expand_path_with(&p, &cwd);
                if expanded.is_dir() {
                    if permission_denied(&p) {
                        #[cfg(unix)]
                        let error_msg = format!(
                            "The permissions of {:o} do not allow access for this user",
                            expanded
                                .metadata()
                                .expect(
                                    "this shouldn't be called since we already know there is a dir"
                                )
                                .permissions()
                                .mode()
                                & 0o0777
                        );
                        #[cfg(not(unix))]
                        let error_msg = String::from("Permission denied");
                        return Err(ShellError::SpannedLabeledError(
                            "Permission denied".to_string(),
                            error_msg,
                            p_tag,
                        ));
                    }
                    if is_empty_dir(&expanded) {
                        return Ok(Value::nothing(call_span).into_pipeline_data());
                    }
                    p.push("*");
                }
                let absolute_path = p.is_absolute();
                (p, p_tag, absolute_path)
            }
            None => {
                if is_empty_dir(current_dir(engine_state, stack)?) {
                    return Ok(Value::nothing(call_span).into_pipeline_data());
                } else {
                    (PathBuf::from("./*"), call_span, false)
                }
            }
        };

        let hidden_dir_specified = is_hidden_dir(&path);

        let glob_path = Spanned {
            item: path.display().to_string(),
            span: p_tag,
        };
        let (prefix, paths) = nu_engine::glob_from(&glob_path, &cwd, call_span)?;

        let mut paths_peek = paths.peekable();
        if paths_peek.peek().is_none() {
            return Err(ShellError::LabeledError(
                format!("No matches found for {}", &path.display().to_string()),
                "no matches found".to_string(),
            ));
        }

        let mut hidden_dirs = vec![];

        Ok(paths_peek
            .into_iter()
            .filter_map(move |x| match x {
                Ok(path) => {
                    let metadata = match std::fs::symlink_metadata(&path) {
                        Ok(metadata) => Some(metadata),
                        Err(_) => None,
                    };
                    if path_contains_hidden_folder(&path, &hidden_dirs) {
                        return None;
                    }

                    if !all && !hidden_dir_specified && is_hidden_dir(&path) {
                        if path.is_dir() {
                            hidden_dirs.push(path);
                        }
                        return None;
                    }

                    let display_name = if short_names {
                        path.file_name().map(|os| os.to_string_lossy().to_string())
                    } else if full_paths || absolute_path {
                        Some(path.to_string_lossy().to_string())
                    } else if let Some(prefix) = &prefix {
                        if let Ok(remainder) = path.strip_prefix(&prefix) {
                            let new_prefix = if let Some(pfx) = diff_paths(&prefix, &cwd) {
                                pfx
                            } else {
                                prefix.to_path_buf()
                            };

                            Some(new_prefix.join(remainder).to_string_lossy().to_string())
                        } else {
                            Some(path.to_string_lossy().to_string())
                        }
                    } else {
                        Some(path.to_string_lossy().to_string())
                    }
                    .ok_or_else(|| {
                        ShellError::SpannedLabeledError(
                            format!("Invalid file name: {:}", path.to_string_lossy()),
                            "invalid file name".into(),
                            call_span,
                        )
                    });

                    match display_name {
                        Ok(name) => {
                            let entry = dir_entry_dict(
                                &path,
                                &name,
                                metadata.as_ref(),
                                call_span,
                                long,
                                du,
                                ctrl_c.clone(),
                            );
                            match entry {
                                Ok(value) => Some(value),
                                Err(err) => Some(Value::Error { error: err }),
                            }
                        }
                        Err(err) => Some(Value::Error { error: err }),
                    }
                }
                _ => Some(Value::Nothing { span: call_span }),
            })
            .into_pipeline_data_with_metadata(
                PipelineMetadata {
                    data_source: DataSource::Ls,
                },
                engine_state.ctrlc.clone(),
            ))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "List all files in the current directory",
                example: "ls",
                result: None,
            },
            Example {
                description: "List all files in a subdirectory",
                example: "ls subdir",
                result: None,
            },
            Example {
                description: "List all rust files",
                example: "ls *.rs",
                result: None,
            },
            Example {
                description: "List all files and directories whose name do not contain 'bar'",
                example: "ls -s | where name !~ bar",
                result: None,
            },
            Example {
                description: "List all dirs with full path name in your home directory",
                example: "ls -f ~ | where type == dir",
                result: None,
            },
            Example {
                description:
                    "List all dirs in your home directory which have not been modified in 7 days",
                example: "ls -s ~ | where type == dir && modified < ((date now) - 7day)",
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

fn is_empty_dir(dir: impl AsRef<Path>) -> bool {
    match dir.as_ref().read_dir() {
        Err(_) => true,
        Ok(mut s) => s.next().is_none(),
    }
}

fn is_hidden_dir(dir: impl AsRef<Path>) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        if let Ok(metadata) = dir.as_ref().metadata() {
            let attributes = metadata.file_attributes();
            // https://docs.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
            (attributes & 0x2) != 0
        } else {
            false
        }
    }

    #[cfg(not(windows))]
    {
        dir.as_ref()
            .file_name()
            .map(|name| name.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
    }
}

fn path_contains_hidden_folder(path: &Path, folders: &[PathBuf]) -> bool {
    let path_str = path.to_str().expect("failed to read path");
    if folders
        .iter()
        .any(|p| path_str.starts_with(&p.to_str().expect("failed to read hidden paths")))
    {
        return true;
    }
    false
}

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::path::Path;
use std::sync::atomic::AtomicBool;

pub fn get_file_type(md: &std::fs::Metadata) -> &str {
    let ft = md.file_type();
    let mut file_type = "unknown";
    if ft.is_dir() {
        file_type = "dir";
    } else if ft.is_file() {
        file_type = "file";
    } else if ft.is_symlink() {
        file_type = "symlink";
    } else {
        #[cfg(unix)]
        {
            if ft.is_block_device() {
                file_type = "block device";
            } else if ft.is_char_device() {
                file_type = "char device";
            } else if ft.is_fifo() {
                file_type = "pipe";
            } else if ft.is_socket() {
                file_type = "socket";
            }
        }
    }
    file_type
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dir_entry_dict(
    filename: &std::path::Path, // absolute path
    display_name: &str,         // gile name to be displayed
    metadata: Option<&std::fs::Metadata>,
    span: Span,
    long: bool,
    du: bool,
    ctrl_c: Option<Arc<AtomicBool>>,
) -> Result<Value, ShellError> {
    let mut cols = vec![];
    let mut vals = vec![];

    cols.push("name".into());
    vals.push(Value::String {
        val: display_name.to_string(),
        span,
    });

    if let Some(md) = metadata {
        cols.push("type".into());
        vals.push(Value::String {
            val: get_file_type(md).to_string(),
            span,
        });
    } else {
        cols.push("type".into());
        vals.push(Value::nothing(span));
    }

    if long {
        cols.push("target".into());
        if let Some(md) = metadata {
            if md.file_type().is_symlink() {
                if let Ok(path_to_link) = filename.read_link() {
                    vals.push(Value::String {
                        val: path_to_link.to_string_lossy().to_string(),
                        span,
                    });
                } else {
                    vals.push(Value::String {
                        val: "Could not obtain target file's path".to_string(),
                        span,
                    });
                }
            } else {
                vals.push(Value::nothing(span));
            }
        }
    }

    if long {
        if let Some(md) = metadata {
            cols.push("readonly".into());
            vals.push(Value::Bool {
                val: md.permissions().readonly(),
                span,
            });

            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let mode = md.permissions().mode();
                cols.push("mode".into());
                vals.push(Value::String {
                    val: umask::Mode::from(mode).to_string(),
                    span,
                });

                let nlinks = md.nlink();
                cols.push("num_links".into());
                vals.push(Value::Int {
                    val: nlinks as i64,
                    span,
                });

                let inode = md.ino();
                cols.push("inode".into());
                vals.push(Value::Int {
                    val: inode as i64,
                    span,
                });

                cols.push("uid".into());
                if let Some(user) = users::get_user_by_uid(md.uid()) {
                    vals.push(Value::String {
                        val: user.name().to_string_lossy().into(),
                        span,
                    });
                } else {
                    vals.push(Value::nothing(span))
                }

                cols.push("group".into());
                if let Some(group) = users::get_group_by_gid(md.gid()) {
                    vals.push(Value::String {
                        val: group.name().to_string_lossy().into(),
                        span,
                    });
                } else {
                    vals.push(Value::nothing(span))
                }
            }
        }
    }

    cols.push("size".to_string());
    if let Some(md) = metadata {
        if md.is_dir() {
            if du {
                let params = DirBuilder::new(Span { start: 0, end: 2 }, None, false, None, false);
                let dir_size = DirInfo::new(filename, &params, None, ctrl_c).get_size();

                vals.push(Value::Filesize {
                    val: dir_size as i64,
                    span,
                });
            } else {
                let dir_size: u64 = md.len();

                vals.push(Value::Filesize {
                    val: dir_size as i64,
                    span,
                });
            };
        } else if md.is_file() {
            vals.push(Value::Filesize {
                val: md.len() as i64,
                span,
            });
        } else if md.file_type().is_symlink() {
            if let Ok(symlink_md) = filename.symlink_metadata() {
                vals.push(Value::Filesize {
                    val: symlink_md.len() as i64,
                    span,
                });
            } else {
                vals.push(Value::nothing(span));
            }
        } else {
            vals.push(Value::nothing(span));
        }
    } else {
        vals.push(Value::nothing(span));
    }

    if let Some(md) = metadata {
        if long {
            cols.push("created".to_string());
            if let Ok(c) = md.created() {
                let utc: DateTime<Local> = c.into();
                vals.push(Value::Date {
                    val: utc.with_timezone(utc.offset()),
                    span,
                });
            } else {
                vals.push(Value::nothing(span));
            }

            cols.push("accessed".to_string());
            if let Ok(a) = md.accessed() {
                let utc: DateTime<Local> = a.into();
                vals.push(Value::Date {
                    val: utc.with_timezone(utc.offset()),
                    span,
                });
            } else {
                vals.push(Value::nothing(span));
            }
        }

        cols.push("modified".to_string());
        if let Ok(m) = md.modified() {
            let utc: DateTime<Local> = m.into();
            vals.push(Value::Date {
                val: utc.with_timezone(utc.offset()),
                span,
            });
        } else {
            vals.push(Value::nothing(span));
        }
    } else {
        if long {
            cols.push("created".to_string());
            vals.push(Value::nothing(span));

            cols.push("accessed".to_string());
            vals.push(Value::nothing(span));
        }

        cols.push("modified".to_string());
        vals.push(Value::nothing(span));
    }

    Ok(Value::Record { cols, vals, span })
}
