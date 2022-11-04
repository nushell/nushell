use crate::DirBuilder;
use crate::DirInfo;
use chrono::{DateTime, Local, LocalResult, TimeZone, Utc};
use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_glob::MatchOptions;
use nu_path::expand_to_real_path;
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
use std::time::{SystemTime, UNIX_EPOCH};

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
                "Get all available columns for each entry (slower; columns are platform-dependent)",
                Some('l'),
            )
            .switch(
                "short-names",
                "Only print the file names, and not the path",
                Some('s'),
            )
            .switch("full-paths", "display paths as absolute paths", Some('f'))
            .switch(
                "du",
                "Display the apparent directory size in place of the directory metadata size",
                Some('d'),
            )
            .switch(
                "directory",
                "List the specified directory itself instead of its contents",
                Some('D'),
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
        let directory = call.has_flag("directory");
        let ctrl_c = engine_state.ctrlc.clone();
        let call_span = call.head;
        let cwd = current_dir(engine_state, stack)?;

        let pattern_arg: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;

        let pattern_arg = {
            if let Some(path) = pattern_arg {
                Some(Spanned {
                    item: nu_utils::strip_ansi_string_unlikely(path.item),
                    span: path.span,
                })
            } else {
                pattern_arg
            }
        };

        let (path, p_tag, absolute_path) = match pattern_arg {
            Some(p) => {
                let p_tag = p.span;
                let mut p = expand_to_real_path(p.item);

                let expanded = nu_path::expand_path_with(&p, &cwd);
                // Avoid checking and pushing "*" to the path when directory (do not show contents) flag is true
                if !directory && expanded.is_dir() {
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
                        return Err(ShellError::GenericError(
                            "Permission denied".to_string(),
                            error_msg,
                            Some(p_tag),
                            None,
                            Vec::new(),
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
                // Avoid pushing "*" to the default path when directory (do not show contents) flag is true
                if directory {
                    (PathBuf::from("."), call_span, false)
                } else if is_empty_dir(current_dir(engine_state, stack)?) {
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

        let glob_options = if all {
            None
        } else {
            let mut glob_options = MatchOptions::new();
            glob_options.recursive_match_hidden_dir = false;
            Some(glob_options)
        };
        let (prefix, paths) = nu_engine::glob_from(&glob_path, &cwd, call_span, glob_options)?;

        let mut paths_peek = paths.peekable();
        if paths_peek.peek().is_none() {
            return Err(ShellError::GenericError(
                format!("No matches found for {}", &path.display().to_string()),
                "".to_string(),
                None,
                Some("no matches found".to_string()),
                Vec::new(),
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
                        if let Ok(remainder) = path.strip_prefix(prefix) {
                            if directory {
                                // When the path is the same as the cwd, path_diff should be "."
                                let path_diff =
                                    if let Some(path_diff_not_dot) = diff_paths(&path, &cwd) {
                                        let path_diff_not_dot = path_diff_not_dot.to_string_lossy();
                                        if path_diff_not_dot.is_empty() {
                                            ".".to_string()
                                        } else {
                                            path_diff_not_dot.to_string()
                                        }
                                    } else {
                                        path.to_string_lossy().to_string()
                                    };

                                Some(path_diff)
                            } else {
                                let new_prefix = if let Some(pfx) = diff_paths(prefix, &cwd) {
                                    pfx
                                } else {
                                    prefix.to_path_buf()
                                };

                                Some(new_prefix.join(remainder).to_string_lossy().to_string())
                            }
                        } else {
                            Some(path.to_string_lossy().to_string())
                        }
                    } else {
                        Some(path.to_string_lossy().to_string())
                    }
                    .ok_or_else(|| {
                        ShellError::GenericError(
                            format!("Invalid file name: {:}", path.to_string_lossy()),
                            "invalid file name".into(),
                            Some(call_span),
                            None,
                            Vec::new(),
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
                description: "List visible files in the current directory",
                example: "ls",
                result: None,
            },
            Example {
                description: "List visible files in a subdirectory",
                example: "ls subdir",
                result: None,
            },
            Example {
                description: "List visible files with full path in the parent directory",
                example: "ls -f ..",
                result: None,
            },
            Example {
                description: "List Rust files",
                example: "ls *.rs",
                result: None,
            },
            Example {
                description: "List files and directories whose name do not contain 'bar'",
                example: "ls -s | where name !~ bar",
                result: None,
            },
            Example {
                description: "List all dirs in your home directory",
                example: "ls -a ~ | where type == dir",
                result: None,
            },
            Example {
                description:
                    "List all dirs in your home directory which have not been modified in 7 days",
                example: "ls -as ~ | where type == dir && modified < ((date now) - 7day)",
                result: None,
            },
            Example {
                description: "List given paths and show directories themselves",
                example: "['/path/to/directory' '/path/to/file'] | each { ls -D $in } | flatten",
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
    if folders.iter().any(|p| path.starts_with(p.as_path())) {
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
    #[cfg(windows)]
    if metadata.is_none() {
        return windows_helper::dir_entry_dict_windows_fallback(filename, display_name, span, long);
    }

    let mut cols = vec![];
    let mut vals = vec![];
    let mut file_type = "unknown";

    cols.push("name".into());
    vals.push(Value::String {
        val: display_name.to_string(),
        span,
    });

    if let Some(md) = metadata {
        file_type = get_file_type(md);
        cols.push("type".into());
        vals.push(Value::String {
            val: file_type.to_string(),
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
                    vals.push(Value::Int {
                        val: md.uid() as i64,
                        span,
                    })
                }

                cols.push("group".into());
                if let Some(group) = users::get_group_by_gid(md.gid()) {
                    vals.push(Value::String {
                        val: group.name().to_string_lossy().into(),
                        span,
                    });
                } else {
                    vals.push(Value::Int {
                        val: md.gid() as i64,
                        span,
                    })
                }
            }
        }
    }

    cols.push("size".to_string());
    if let Some(md) = metadata {
        let zero_sized = file_type == "pipe"
            || file_type == "socket"
            || file_type == "char device"
            || file_type == "block device";

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
            let value = if zero_sized {
                Value::Filesize { val: 0, span }
            } else {
                Value::nothing(span)
            };
            vals.push(value);
        }
    } else {
        vals.push(Value::nothing(span));
    }

    if let Some(md) = metadata {
        if long {
            cols.push("created".to_string());
            {
                let mut val = Value::nothing(span);
                if let Ok(c) = md.created() {
                    if let Some(local) = try_convert_to_local_date_time(c) {
                        val = Value::Date {
                            val: local.with_timezone(local.offset()),
                            span,
                        };
                    }
                }
                vals.push(val);
            }

            cols.push("accessed".to_string());
            {
                let mut val = Value::nothing(span);
                if let Ok(a) = md.accessed() {
                    if let Some(local) = try_convert_to_local_date_time(a) {
                        val = Value::Date {
                            val: local.with_timezone(local.offset()),
                            span,
                        };
                    }
                }
                vals.push(val);
            }
        }

        cols.push("modified".to_string());
        {
            let mut val = Value::nothing(span);
            if let Ok(m) = md.modified() {
                if let Some(local) = try_convert_to_local_date_time(m) {
                    val = Value::Date {
                        val: local.with_timezone(local.offset()),
                        span,
                    };
                }
            }
            vals.push(val);
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

// TODO: can we get away from local times in `ls`? internals might be cleaner if we worked in UTC
// and left the conversion to local time to the display layer
fn try_convert_to_local_date_time(t: SystemTime) -> Option<DateTime<Local>> {
    // Adapted from https://github.com/chronotope/chrono/blob/v0.4.19/src/datetime.rs#L755-L767.
    let (sec, nsec) = match t.duration_since(UNIX_EPOCH) {
        Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
        Err(e) => {
            // unlikely but should be handled
            let dur = e.duration();
            let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
            if nsec == 0 {
                (-sec, 0)
            } else {
                (-sec - 1, 1_000_000_000 - nsec)
            }
        }
    };

    match Utc.timestamp_opt(sec, nsec) {
        LocalResult::Single(t) => Some(t.with_timezone(&Local)),
        _ => None,
    }
}

// #[cfg(windows)] is just to make Clippy happy, remove if you ever want to use this on other platforms
#[cfg(windows)]
fn unix_time_to_local_date_time(secs: i64) -> Option<DateTime<Local>> {
    match Utc.timestamp_opt(secs, 0) {
        LocalResult::Single(t) => Some(t.with_timezone(&Local)),
        _ => None,
    }
}

#[cfg(windows)]
mod windows_helper {
    use super::*;

    use std::mem::MaybeUninit;
    use std::os::windows::prelude::OsStrExt;
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::Storage::FileSystem::{
        FindFirstFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY,
        FILE_ATTRIBUTE_REPARSE_POINT, WIN32_FIND_DATAW,
    };
    use windows::Win32::System::SystemServices::{
        IO_REPARSE_TAG_MOUNT_POINT, IO_REPARSE_TAG_SYMLINK,
    };

    /// A secondary way to get file info on Windows, for when std::fs::symlink_metadata() fails.
    /// dir_entry_dict depends on metadata, but that can't be retrieved for some Windows system files:
    /// https://github.com/rust-lang/rust/issues/96980
    pub fn dir_entry_dict_windows_fallback(
        filename: &Path,
        display_name: &str,
        span: Span,
        long: bool,
    ) -> Result<Value, ShellError> {
        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("name".into());
        vals.push(Value::String {
            val: display_name.to_string(),
            span,
        });

        let find_data = find_first_file(filename, span)?;

        cols.push("type".into());
        vals.push(Value::String {
            val: get_file_type_windows_fallback(&find_data),
            span,
        });

        if long {
            cols.push("target".into());
            if is_symlink(&find_data) {
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

            cols.push("readonly".into());
            vals.push(Value::Bool {
                val: (find_data.dwFileAttributes & FILE_ATTRIBUTE_READONLY.0 != 0),
                span,
            });
        }

        cols.push("size".to_string());
        let file_size = (find_data.nFileSizeHigh as u64) << 32 | find_data.nFileSizeLow as u64;
        vals.push(Value::Filesize {
            val: file_size as i64,
            span,
        });

        if long {
            cols.push("created".to_string());
            {
                let mut val = Value::nothing(span);
                let seconds_since_unix_epoch = unix_time_from_filetime(&find_data.ftCreationTime);
                if let Some(local) = unix_time_to_local_date_time(seconds_since_unix_epoch) {
                    val = Value::Date {
                        val: local.with_timezone(local.offset()),
                        span,
                    };
                }
                vals.push(val);
            }

            cols.push("accessed".to_string());
            {
                let mut val = Value::nothing(span);
                let seconds_since_unix_epoch = unix_time_from_filetime(&find_data.ftLastAccessTime);
                if let Some(local) = unix_time_to_local_date_time(seconds_since_unix_epoch) {
                    val = Value::Date {
                        val: local.with_timezone(local.offset()),
                        span,
                    };
                }
                vals.push(val);
            }
        }

        cols.push("modified".to_string());
        {
            let mut val = Value::nothing(span);
            let seconds_since_unix_epoch = unix_time_from_filetime(&find_data.ftLastWriteTime);
            if let Some(local) = unix_time_to_local_date_time(seconds_since_unix_epoch) {
                val = Value::Date {
                    val: local.with_timezone(local.offset()),
                    span,
                };
            }
            vals.push(val);
        }

        Ok(Value::Record { cols, vals, span })
    }

    fn unix_time_from_filetime(ft: &FILETIME) -> i64 {
        /// January 1, 1970 as Windows file time
        const EPOCH_AS_FILETIME: u64 = 116444736000000000;
        const HUNDREDS_OF_NANOSECONDS: u64 = 10000000;

        let time_u64 = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
        let rel_to_linux_epoch = time_u64 - EPOCH_AS_FILETIME;
        let seconds_since_unix_epoch = rel_to_linux_epoch / HUNDREDS_OF_NANOSECONDS;

        seconds_since_unix_epoch as i64
    }

    // wrapper around the FindFirstFileW Win32 API
    fn find_first_file(filename: &Path, span: Span) -> Result<WIN32_FIND_DATAW, ShellError> {
        unsafe {
            let mut find_data = MaybeUninit::<WIN32_FIND_DATAW>::uninit();
            // The windows crate really needs a nicer way to do string conversions
            let filename_wide: Vec<u16> = filename
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            if FindFirstFileW(
                windows::core::PCWSTR(filename_wide.as_ptr()),
                find_data.as_mut_ptr(),
            )
            .is_err()
            {
                return Err(ShellError::ReadingFile(
                    "Could not read file metadata".to_string(),
                    span,
                ));
            }

            let find_data = find_data.assume_init();
            Ok(find_data)
        }
    }

    fn get_file_type_windows_fallback(find_data: &WIN32_FIND_DATAW) -> String {
        if find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0 {
            return "dir".to_string();
        }

        if is_symlink(find_data) {
            return "symlink".to_string();
        }

        "file".to_string()
    }

    fn is_symlink(find_data: &WIN32_FIND_DATAW) -> bool {
        if find_data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            // Follow Golang's lead in treating mount points as symlinks.
            // https://github.com/golang/go/blob/016d7552138077741a9c3fdadc73c0179f5d3ff7/src/os/types_windows.go#L104-L105
            if find_data.dwReserved0 == IO_REPARSE_TAG_SYMLINK
                || find_data.dwReserved0 == IO_REPARSE_TAG_MOUNT_POINT
            {
                return true;
            }
        }
        false
    }
}
