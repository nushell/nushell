use crate::{DirBuilder, DirInfo};
use chrono::{DateTime, Local, LocalResult, TimeZone, Utc};
use nu_engine::glob_from;
#[allow(deprecated)]
use nu_engine::{command_prelude::*, env::current_dir};
use nu_glob::MatchOptions;
use nu_path::{expand_path_with, expand_to_real_path};
use nu_protocol::{
    DataSource, NuGlob, PipelineMetadata, Signals,
    shell_error::{self, io::IoError},
};
use pathdiff::diff_paths;
use rayon::prelude::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    cmp::Ordering,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct Ls;

#[derive(Clone, Copy)]
struct Args {
    all: bool,
    long: bool,
    short_names: bool,
    full_paths: bool,
    du: bool,
    directory: bool,
    use_mime_type: bool,
    use_threads: bool,
    call_span: Span,
}

impl Command for Ls {
    fn name(&self) -> &str {
        "ls"
    }

    fn description(&self) -> &str {
        "List the filenames, sizes, and modification times of items in a directory."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["dir"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("ls")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            // LsGlobPattern is similar to string, it won't auto-expand
            // and we use it to track if the user input is quoted.
            .rest("pattern", SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::String]), "The glob pattern to use.")
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
                "Display the apparent directory size (\"disk usage\") in place of the directory metadata size",
                Some('d'),
            )
            .switch(
                "directory",
                "List the specified directory itself instead of its contents",
                Some('D'),
            )
            .switch("mime-type", "Show mime-type in type column instead of 'file' (based on filenames only; files' contents are not examined)", Some('m'))
            .switch("threads", "Use multiple threads to list contents. Output will be non-deterministic.", Some('t'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let all = call.has_flag(engine_state, stack, "all")?;
        let long = call.has_flag(engine_state, stack, "long")?;
        let short_names = call.has_flag(engine_state, stack, "short-names")?;
        let full_paths = call.has_flag(engine_state, stack, "full-paths")?;
        let du = call.has_flag(engine_state, stack, "du")?;
        let directory = call.has_flag(engine_state, stack, "directory")?;
        let use_mime_type = call.has_flag(engine_state, stack, "mime-type")?;
        let use_threads = call.has_flag(engine_state, stack, "threads")?;
        let call_span = call.head;
        #[allow(deprecated)]
        let cwd = current_dir(engine_state, stack)?;

        let args = Args {
            all,
            long,
            short_names,
            full_paths,
            du,
            directory,
            use_mime_type,
            use_threads,
            call_span,
        };

        let pattern_arg = call.rest::<Spanned<NuGlob>>(engine_state, stack, 0)?;
        let input_pattern_arg = if !call.has_positional_args(stack, 0) {
            None
        } else {
            Some(pattern_arg)
        };
        match input_pattern_arg {
            None => Ok(
                ls_for_one_pattern(None, args, engine_state.signals().clone(), cwd)?
                    .into_pipeline_data_with_metadata(
                        call_span,
                        engine_state.signals().clone(),
                        PipelineMetadata {
                            data_source: DataSource::Ls,
                            ..Default::default()
                        },
                    ),
            ),
            Some(pattern) => {
                let mut result_iters = vec![];
                for pat in pattern {
                    result_iters.push(ls_for_one_pattern(
                        Some(pat),
                        args,
                        engine_state.signals().clone(),
                        cwd.clone(),
                    )?)
                }

                // Here nushell needs to use
                // use `flatten` to chain all iterators into one.
                Ok(result_iters
                    .into_iter()
                    .flatten()
                    .into_pipeline_data_with_metadata(
                        call_span,
                        engine_state.signals().clone(),
                        PipelineMetadata {
                            data_source: DataSource::Ls,
                            ..Default::default()
                        },
                    ))
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
                example: "ls | where name !~ bar",
                result: None,
            },
            Example {
                description: "List the full path of all dirs in your home directory",
                example: "ls -a ~ | where type == dir",
                result: None,
            },
            Example {
                description: "List only the names (not paths) of all dirs in your home directory which have not been modified in 7 days",
                example: "ls -as ~ | where type == dir and modified < ((date now) - 7day)",
                result: None,
            },
            Example {
                description: "Recursively list all files and subdirectories under the current directory using a glob pattern",
                example: "ls -a **/*",
                result: None,
            },
            Example {
                description: "Recursively list *.rs and *.toml files using the glob command",
                example: "ls ...(glob **/*.{rs,toml})",
                result: None,
            },
            Example {
                description: "List given paths and show directories themselves",
                example: "['/path/to/directory' '/path/to/file'] | each {|| ls -D $in } | flatten",
                result: None,
            },
        ]
    }
}

fn ls_for_one_pattern(
    pattern_arg: Option<Spanned<NuGlob>>,
    args: Args,
    signals: Signals,
    cwd: PathBuf,
) -> Result<PipelineData, ShellError> {
    fn create_pool(num_threads: usize) -> Result<rayon::ThreadPool, ShellError> {
        match rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
        {
            Err(e) => Err(e).map_err(|e| ShellError::GenericError {
                error: "Error creating thread pool".into(),
                msg: e.to_string(),
                span: Some(Span::unknown()),
                help: None,
                inner: vec![],
            }),
            Ok(pool) => Ok(pool),
        }
    }

    let (tx, rx) = mpsc::channel();

    let Args {
        all,
        long,
        short_names,
        full_paths,
        du,
        directory,
        use_mime_type,
        use_threads,
        call_span,
    } = args;
    let pattern_arg = {
        if let Some(path) = pattern_arg {
            // it makes no sense to list an empty string.
            if path.item.as_ref().is_empty() {
                return Err(ShellError::Io(IoError::new_with_additional_context(
                    shell_error::io::ErrorKind::from_std(std::io::ErrorKind::NotFound),
                    path.span,
                    PathBuf::from(path.item.to_string()),
                    "empty string('') directory or file does not exist",
                )));
            }
            match path.item {
                NuGlob::DoNotExpand(p) => Some(Spanned {
                    item: NuGlob::DoNotExpand(nu_utils::strip_ansi_string_unlikely(p)),
                    span: path.span,
                }),
                NuGlob::Expand(p) => Some(Spanned {
                    item: NuGlob::Expand(nu_utils::strip_ansi_string_unlikely(p)),
                    span: path.span,
                }),
            }
        } else {
            pattern_arg
        }
    };

    let mut just_read_dir = false;
    let p_tag: Span = pattern_arg.as_ref().map(|p| p.span).unwrap_or(call_span);
    let (pattern_arg, absolute_path) = match pattern_arg {
        Some(pat) => {
            // expand with cwd here is only used for checking
            let tmp_expanded =
                nu_path::expand_path_with(pat.item.as_ref(), &cwd, pat.item.is_expand());
            // Avoid checking and pushing "*" to the path when directory (do not show contents) flag is true
            if !directory && tmp_expanded.is_dir() {
                if read_dir(tmp_expanded, p_tag, use_threads, signals.clone())?
                    .next()
                    .is_none()
                {
                    return Ok(Value::test_nothing().into_pipeline_data());
                }
                just_read_dir = !(pat.item.is_expand() && nu_glob::is_glob(pat.item.as_ref()));
            }

            // it's absolute path if:
            // 1. pattern is absolute.
            // 2. pattern can be expanded, and after expands to real_path, it's absolute.
            //    here `expand_to_real_path` call is required, because `~/aaa` should be absolute
            //    path.
            let absolute_path = Path::new(pat.item.as_ref()).is_absolute()
                || (pat.item.is_expand() && expand_to_real_path(pat.item.as_ref()).is_absolute());
            (pat.item, absolute_path)
        }
        None => {
            // Avoid pushing "*" to the default path when directory (do not show contents) flag is true
            if directory {
                (NuGlob::Expand(".".to_string()), false)
            } else if read_dir(cwd.clone(), p_tag, use_threads, signals.clone())?
                .next()
                .is_none()
            {
                return Ok(Value::test_nothing().into_pipeline_data());
            } else {
                (NuGlob::Expand("*".to_string()), false)
            }
        }
    };

    let hidden_dir_specified = is_hidden_dir(pattern_arg.as_ref());

    let path = pattern_arg.into_spanned(p_tag);
    let (prefix, paths) = if just_read_dir {
        let expanded = nu_path::expand_path_with(path.item.as_ref(), &cwd, path.item.is_expand());
        let paths = read_dir(expanded.clone(), p_tag, use_threads, signals.clone())?;
        // just need to read the directory, so prefix is path itself.
        (Some(expanded), paths)
    } else {
        let glob_options = if all {
            None
        } else {
            let glob_options = MatchOptions {
                recursive_match_hidden_dir: false,
                ..Default::default()
            };
            Some(glob_options)
        };
        glob_from(&path, &cwd, call_span, glob_options, signals.clone())?
    };

    let mut paths_peek = paths.peekable();
    let no_matches = paths_peek.peek().is_none();
    signals.check(&call_span)?;
    if no_matches {
        return Err(ShellError::GenericError {
            error: format!("No matches found for {:?}", path.item),
            msg: "Pattern, file or folder not found".into(),
            span: Some(p_tag),
            help: Some("no matches found".into()),
            inner: vec![],
        });
    }

    let hidden_dirs = Arc::new(Mutex::new(Vec::new()));

    let signals_clone = signals.clone();

    let pool = if use_threads {
        let count = std::thread::available_parallelism()
            .map_err(|err| {
                IoError::new_with_additional_context(
                    err,
                    call_span,
                    None,
                    "Could not get available parallelism",
                )
            })?
            .get();
        create_pool(count)?
    } else {
        create_pool(1)?
    };

    pool.install(|| {
        rayon::spawn(move || {
            let result = paths_peek
                .par_bridge()
                .filter_map(move |x| match x {
                    Ok(path) => {
                        let metadata = std::fs::symlink_metadata(&path).ok();
                        let hidden_dir_clone = Arc::clone(&hidden_dirs);
                        let mut hidden_dir_mutex = hidden_dir_clone
                            .lock()
                            .expect("Unable to acquire lock for hidden_dirs");
                        if path_contains_hidden_folder(&path, &hidden_dir_mutex) {
                            return None;
                        }

                        if !all && !hidden_dir_specified && is_hidden_dir(&path) {
                            if path.is_dir() {
                                hidden_dir_mutex.push(path);
                                drop(hidden_dir_mutex);
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
                                    let path_diff = if let Some(path_diff_not_dot) =
                                        diff_paths(&path, &cwd)
                                    {
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
                        .ok_or_else(|| ShellError::GenericError {
                            error: format!("Invalid file name: {:}", path.to_string_lossy()),
                            msg: "invalid file name".into(),
                            span: Some(call_span),
                            help: None,
                            inner: vec![],
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
                                    &signals_clone,
                                    use_mime_type,
                                    args.full_paths,
                                );
                                match entry {
                                    Ok(value) => Some(value),
                                    Err(err) => Some(Value::error(err, call_span)),
                                }
                            }
                            Err(err) => Some(Value::error(err, call_span)),
                        }
                    }
                    Err(err) => Some(Value::error(err, call_span)),
                })
                .try_for_each(|stream| {
                    tx.send(stream).map_err(|e| ShellError::GenericError {
                        error: "Error streaming data".into(),
                        msg: e.to_string(),
                        span: Some(call_span),
                        help: None,
                        inner: vec![],
                    })
                })
                .map_err(|err| ShellError::GenericError {
                    error: "Unable to create a rayon pool".into(),
                    msg: err.to_string(),
                    span: Some(call_span),
                    help: None,
                    inner: vec![],
                });

            if let Err(error) = result {
                let _ = tx.send(Value::error(error, call_span));
            }
        });
    });

    Ok(rx
        .into_iter()
        .into_pipeline_data(call_span, signals.clone()))
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

pub fn get_file_type(md: &std::fs::Metadata, display_name: &str, use_mime_type: bool) -> String {
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
    if use_mime_type {
        let guess = mime_guess::from_path(display_name);
        let mime_guess = match guess.first() {
            Some(mime_type) => mime_type.essence_str().to_string(),
            None => "unknown".to_string(),
        };
        if file_type == "file" {
            mime_guess
        } else {
            file_type.to_string()
        }
    } else {
        file_type.to_string()
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dir_entry_dict(
    filename: &std::path::Path, // absolute path
    display_name: &str,         // file name to be displayed
    metadata: Option<&std::fs::Metadata>,
    span: Span,
    long: bool,
    du: bool,
    signals: &Signals,
    use_mime_type: bool,
    full_symlink_target: bool,
) -> Result<Value, ShellError> {
    #[cfg(windows)]
    if metadata.is_none() {
        return Ok(windows_helper::dir_entry_dict_windows_fallback(
            filename,
            display_name,
            span,
            long,
        ));
    }

    let mut record = Record::new();
    let mut file_type = "unknown".to_string();

    record.push("name", Value::string(display_name, span));

    if let Some(md) = metadata {
        file_type = get_file_type(md, display_name, use_mime_type);
        record.push("type", Value::string(file_type.clone(), span));
    } else {
        record.push("type", Value::nothing(span));
    }

    if long && let Some(md) = metadata {
        record.push(
            "target",
            if md.file_type().is_symlink() {
                if let Ok(path_to_link) = filename.read_link() {
                    // Actually `filename` should always have a parent because it's a symlink.
                    // But for safety, we check `filename.parent().is_some()` first.
                    if full_symlink_target && filename.parent().is_some() {
                        Value::string(
                            expand_path_with(
                                path_to_link,
                                filename
                                    .parent()
                                    .expect("already check the filename have a parent"),
                                true,
                            )
                            .to_string_lossy(),
                            span,
                        )
                    } else {
                        Value::string(path_to_link.to_string_lossy(), span)
                    }
                } else {
                    Value::string("Could not obtain target file's path", span)
                }
            } else {
                Value::nothing(span)
            },
        )
    }

    if long && let Some(md) = metadata {
        record.push("readonly", Value::bool(md.permissions().readonly(), span));

        #[cfg(unix)]
        {
            use nu_utils::filesystem::users;
            use std::os::unix::fs::MetadataExt;

            let mode = md.permissions().mode();
            record.push(
                "mode",
                Value::string(umask::Mode::from(mode).to_string(), span),
            );

            let nlinks = md.nlink();
            record.push("num_links", Value::int(nlinks as i64, span));

            let inode = md.ino();
            record.push("inode", Value::int(inode as i64, span));

            record.push(
                "user",
                if let Some(user) = users::get_user_by_uid(md.uid().into()) {
                    Value::string(user.name, span)
                } else {
                    Value::int(md.uid().into(), span)
                },
            );

            record.push(
                "group",
                if let Some(group) = users::get_group_by_gid(md.gid().into()) {
                    Value::string(group.name, span)
                } else {
                    Value::int(md.gid().into(), span)
                },
            );
        }
    }

    record.push(
        "size",
        if let Some(md) = metadata {
            let zero_sized = file_type == "pipe"
                || file_type == "socket"
                || file_type == "char device"
                || file_type == "block device";

            if md.is_dir() {
                if du {
                    let params = DirBuilder::new(Span::new(0, 2), None, false, None, false);
                    let dir_size = DirInfo::new(filename, &params, None, span, signals)?.get_size();

                    Value::filesize(dir_size as i64, span)
                } else {
                    let dir_size: u64 = md.len();

                    Value::filesize(dir_size as i64, span)
                }
            } else if md.is_file() {
                Value::filesize(md.len() as i64, span)
            } else if md.file_type().is_symlink() {
                if let Ok(symlink_md) = filename.symlink_metadata() {
                    Value::filesize(symlink_md.len() as i64, span)
                } else {
                    Value::nothing(span)
                }
            } else if zero_sized {
                Value::filesize(0, span)
            } else {
                Value::nothing(span)
            }
        } else {
            Value::nothing(span)
        },
    );

    if let Some(md) = metadata {
        if long {
            record.push("created", {
                let mut val = Value::nothing(span);
                if let Ok(c) = md.created()
                    && let Some(local) = try_convert_to_local_date_time(c)
                {
                    val = Value::date(local.with_timezone(local.offset()), span);
                }
                val
            });

            record.push("accessed", {
                let mut val = Value::nothing(span);
                if let Ok(a) = md.accessed()
                    && let Some(local) = try_convert_to_local_date_time(a)
                {
                    val = Value::date(local.with_timezone(local.offset()), span)
                }
                val
            });
        }

        record.push("modified", {
            let mut val = Value::nothing(span);
            if let Ok(m) = md.modified()
                && let Some(local) = try_convert_to_local_date_time(m)
            {
                val = Value::date(local.with_timezone(local.offset()), span);
            }
            val
        })
    } else {
        if long {
            record.push("created", Value::nothing(span));
            record.push("accessed", Value::nothing(span));
        }

        record.push("modified", Value::nothing(span));
    }

    Ok(Value::record(record, span))
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

    const NEG_UNIX_EPOCH: i64 = -11644473600; // t was invalid 0, UNIX_EPOCH subtracted above.
    if sec == NEG_UNIX_EPOCH {
        // do not tz lookup invalid SystemTime
        return None;
    }
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

    use nu_protocol::shell_error;
    use std::os::windows::prelude::OsStrExt;
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT, FindClose,
        FindFirstFileW, WIN32_FIND_DATAW,
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
    ) -> Value {
        let mut record = Record::new();

        record.push("name", Value::string(display_name, span));

        let find_data = match find_first_file(filename, span) {
            Ok(fd) => fd,
            Err(e) => {
                // Sometimes this happens when the file name is not allowed on Windows (ex: ends with a '.', pipes)
                // For now, we just log it and give up on returning metadata columns
                // TODO: find another way to get this data (like cmd.exe, pwsh, and MINGW bash can)
                log::error!("ls: '{}' {}", filename.to_string_lossy(), e);
                return Value::record(record, span);
            }
        };

        record.push(
            "type",
            Value::string(get_file_type_windows_fallback(&find_data), span),
        );

        if long {
            record.push(
                "target",
                if is_symlink(&find_data) {
                    if let Ok(path_to_link) = filename.read_link() {
                        Value::string(path_to_link.to_string_lossy(), span)
                    } else {
                        Value::string("Could not obtain target file's path", span)
                    }
                } else {
                    Value::nothing(span)
                },
            );

            record.push(
                "readonly",
                Value::bool(
                    find_data.dwFileAttributes & FILE_ATTRIBUTE_READONLY.0 != 0,
                    span,
                ),
            );
        }

        let file_size = ((find_data.nFileSizeHigh as u64) << 32) | find_data.nFileSizeLow as u64;
        record.push("size", Value::filesize(file_size as i64, span));

        if long {
            record.push("created", {
                let mut val = Value::nothing(span);
                let seconds_since_unix_epoch = unix_time_from_filetime(&find_data.ftCreationTime);
                if let Some(local) = unix_time_to_local_date_time(seconds_since_unix_epoch) {
                    val = Value::date(local.with_timezone(local.offset()), span);
                }
                val
            });

            record.push("accessed", {
                let mut val = Value::nothing(span);
                let seconds_since_unix_epoch = unix_time_from_filetime(&find_data.ftLastAccessTime);
                if let Some(local) = unix_time_to_local_date_time(seconds_since_unix_epoch) {
                    val = Value::date(local.with_timezone(local.offset()), span);
                }
                val
            });
        }

        record.push("modified", {
            let mut val = Value::nothing(span);
            let seconds_since_unix_epoch = unix_time_from_filetime(&find_data.ftLastWriteTime);
            if let Some(local) = unix_time_to_local_date_time(seconds_since_unix_epoch) {
                val = Value::date(local.with_timezone(local.offset()), span);
            }
            val
        });

        Value::record(record, span)
    }

    fn unix_time_from_filetime(ft: &FILETIME) -> i64 {
        /// January 1, 1970 as Windows file time
        const EPOCH_AS_FILETIME: u64 = 116444736000000000;
        const HUNDREDS_OF_NANOSECONDS: u64 = 10000000;

        let time_u64 = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
        if time_u64 > 0 {
            let rel_to_linux_epoch = time_u64.saturating_sub(EPOCH_AS_FILETIME);
            let seconds_since_unix_epoch = rel_to_linux_epoch / HUNDREDS_OF_NANOSECONDS;
            return seconds_since_unix_epoch as i64;
        }
        0
    }

    // wrapper around the FindFirstFileW Win32 API
    fn find_first_file(filename: &Path, span: Span) -> Result<WIN32_FIND_DATAW, ShellError> {
        unsafe {
            let mut find_data = WIN32_FIND_DATAW::default();
            // The windows crate really needs a nicer way to do string conversions
            let filename_wide: Vec<u16> = filename
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            match FindFirstFileW(
                windows::core::PCWSTR(filename_wide.as_ptr()),
                &mut find_data,
            ) {
                Ok(handle) => {
                    // Don't forget to close the Find handle
                    // https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-findfirstfilew#remarks
                    // Assumption: WIN32_FIND_DATAW is a pure data struct, so we can let our
                    // find_data outlive the handle.
                    let _ = FindClose(handle);
                    Ok(find_data)
                }
                Err(e) => Err(ShellError::Io(IoError::new_with_additional_context(
                    shell_error::io::ErrorKind::from_std(std::io::ErrorKind::Other),
                    span,
                    PathBuf::from(filename),
                    format!("Could not read metadata: {e}"),
                ))),
            }
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

#[allow(clippy::type_complexity)]
fn read_dir(
    f: PathBuf,
    span: Span,
    use_threads: bool,
    signals: Signals,
) -> Result<Box<dyn Iterator<Item = Result<PathBuf, ShellError>> + Send>, ShellError> {
    let signals_clone = signals.clone();
    let items = f
        .read_dir()
        .map_err(|err| IoError::new(err, span, f.clone()))?
        .map(move |d| {
            signals_clone.check(&span)?;
            d.map(|r| r.path())
                .map_err(|err| IoError::new(err, span, f.clone()))
                .map_err(ShellError::from)
        });
    if !use_threads {
        let mut collected = items.collect::<Vec<_>>();
        signals.check(&span)?;
        collected.sort_by(|a, b| match (a, b) {
            (Ok(a), Ok(b)) => a.cmp(b),
            (Ok(_), Err(_)) => Ordering::Greater,
            (Err(_), Ok(_)) => Ordering::Less,
            (Err(_), Err(_)) => Ordering::Equal,
        });
        return Ok(Box::new(collected.into_iter()));
    }
    Ok(Box::new(items))
}
