//! In-process file indexing runtime for the `idx` command family.
//!
//! The runtime wraps a single `fff-search` [`FilePicker`] and is stored as a thread-safe singleton shared by every idx subcommand.

use chrono::{Local, TimeZone, Utc};
use fff_search::{
    FFFMode, FilePicker, FilePickerOptions, FuzzySearchOptions, GrepConfig, GrepMode,
    GrepSearchOptions, MixedItemRef, PaginationArgs, QueryParser, SharedFilePicker, SharedFrecency,
    watch::{WatchEvent, WatchId, WatchOptions},
};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{ListStream, PipelineMetadata, Signals};
use nu_utils::time::Instant;
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Global in-process runtime for idx commands.
///
/// The runtime is shared by all idx subcommands and backed by one live `fff-search` picker.
#[derive(Clone)]
pub struct IdxRuntime {
    pub base_path: PathBuf,
    pub watch: bool,
    pub shared_picker: SharedFilePicker,
    pub scan_start_time: Instant,
    pub scan_completed: Arc<AtomicBool>,
    pub scan_duration_ns: Arc<AtomicU64>,
}

#[derive(Clone, Debug, Default)]
/// User-facing runtime status for `idx status`.
pub struct IdxStatus {
    pub initialized: bool,
    pub base_path: String,
    pub watch: bool,
    pub scanning: bool,
    pub scan_duration_ns: u64,
    pub files: usize,
    pub dirs: usize,
    pub arena_bytes_base: usize,
    pub arena_bytes_overflow: usize,
    pub arena_bytes_untracked: usize,
}

impl IdxStatus {
    /// Convert status to Nushell record output.
    pub fn to_value(&self, span: Span) -> Value {
        let arena_bytes_total = self
            .arena_bytes_base
            .saturating_add(self.arena_bytes_overflow)
            .saturating_add(self.arena_bytes_untracked);

        Value::record(
            record! {
                "initialized" => Value::bool(self.initialized, span),
                "base_path" => Value::string(self.base_path.clone(), span),
                "watch" => Value::bool(self.watch, span),
                "scanning" => Value::bool(self.scanning, span),
                "scan_duration" => Value::duration(i64::try_from(self.scan_duration_ns).unwrap_or(i64::MAX), span),
                "files" => Value::int(usize_to_i64(self.files), span),
                "dirs" => Value::int(usize_to_i64(self.dirs), span),
                "arena_size_base" => Value::filesize(usize_to_i64(self.arena_bytes_base), span),
                "arena_size_overflow" => Value::filesize(usize_to_i64(self.arena_bytes_overflow), span),
                "arena_size_untracked" => Value::filesize(usize_to_i64(self.arena_bytes_untracked), span),
                "arena_size_total" => Value::filesize(usize_to_i64(arena_bytes_total), span),
            },
            span,
        )
    }
}

static IDX_RUNTIME: OnceLock<Mutex<Option<IdxRuntime>>> = OnceLock::new();

fn runtime() -> &'static Mutex<Option<IdxRuntime>> {
    IDX_RUNTIME.get_or_init(|| Mutex::new(None))
}

/// Convert a `fff_search` error to a Nushell [`ShellError`].
fn fff_error<E: std::fmt::Display>(err: E, span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(
        "idx operation failed",
        err.to_string(),
        span,
    ))
}

/// Error when the idx runtime has not been initialized.
fn idx_not_initialized_error(span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(
        "idx is not initialized",
        "run `idx init <path>` first",
        span,
    ))
}

/// Read lock on the shared FilePicker, returning a guard that dereferences to `Option<FilePicker>`.
fn read_picker_guard<'a>(
    shared_picker: &'a SharedFilePicker,
    span: Span,
) -> Result<impl std::ops::Deref<Target = Option<FilePicker>> + 'a, ShellError> {
    shared_picker.read().map_err(|err| fff_error(err, span))
}

/// Extract the FilePicker reference from a guard, or error if not initialized.
fn picker_from_guard(guard: &Option<FilePicker>, span: Span) -> Result<&FilePicker, ShellError> {
    guard
        .as_ref()
        .ok_or_else(|| idx_not_initialized_error(span))
}

/// Validate that the picker has been initialized before constructing a lazy iterator that cannot return a command-level error.
fn ensure_picker_initialized(
    shared_picker: &SharedFilePicker,
    span: Span,
) -> Result<(), ShellError> {
    let guard = read_picker_guard(shared_picker, span)?;
    let _ = picker_from_guard(&guard, span)?;
    Ok(())
}

/// Shut down the shared FilePicker and its background workers cleanly.
///
/// # Important
///
/// Never hold the shared picker write lock while joining the watcher thread, otherwise the owner thread can deadlock waiting for
/// the same lock while processing a final event batch.
fn shutdown_shared_picker(shared_picker: &SharedFilePicker, span: Span) -> Result<(), ShellError> {
    let mut picker_to_stop = {
        let mut guard = shared_picker.write().map_err(|err| fff_error(err, span))?;
        guard.take()
    };

    if let Some(picker) = picker_to_stop.as_mut() {
        // Stop new work before synchronously joining the watcher owner thread.
        picker.cancel();
        picker.stop_background_monitor();
    }

    Ok(())
}

/// Build an `IdxStatus` from the current FilePicker state.
fn idx_status_from_picker(
    base_path: &Path,
    watch: bool,
    picker: &FilePicker,
    scan_duration_ns: u64,
) -> IdxStatus {
    let (arena_bytes_base, arena_bytes_overflow, arena_bytes_untracked) = picker.arena_bytes();

    IdxStatus {
        initialized: true,
        base_path: base_path.display().to_string(),
        watch,
        scanning: picker.is_scan_active(),
        scan_duration_ns,
        files: picker.live_file_count(),
        dirs: picker
            .get_dirs()
            .iter()
            .filter(|item| !item.is_deleted())
            .count(),
        arena_bytes_base,
        arena_bytes_overflow,
        arena_bytes_untracked,
    }
}

/// Compute the scan duration, freezing it once the scan completes.
///
/// This uses atomic operations to ensure the completion time is recorded exactly once, even if called concurrently.
fn freeze_scan_duration_if_needed(
    scan_completed: &AtomicBool,
    scan_duration_ns: &AtomicU64,
    scan_start: Instant,
    scanning: bool,
) -> u64 {
    if scan_completed.load(Ordering::Acquire) {
        return scan_duration_ns.load(Ordering::Acquire);
    }

    let elapsed = u64::try_from(scan_start.elapsed().as_nanos()).unwrap_or(u64::MAX);
    if !scanning {
        if scan_completed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            scan_duration_ns.store(elapsed, Ordering::Release);
            return elapsed;
        }

        return scan_duration_ns.load(Ordering::Acquire);
    }

    elapsed
}

/// Poll until the background scan completes or times out.
fn wait_for_scan_completion(
    shared_picker: &SharedFilePicker,
    timeout: Duration,
    span: Span,
) -> Result<(), ShellError> {
    let poll_interval = Duration::from_millis(10);
    let startup_grace = Duration::from_millis(250);
    let wait_start = Instant::now();
    let mut saw_active_scan = false;

    loop {
        let guard = read_picker_guard(shared_picker, span)?;
        let picker = picker_from_guard(&guard, span)?;
        let scanning = picker.is_scan_active();
        drop(guard);

        if scanning {
            saw_active_scan = true;
        }

        if saw_active_scan && !scanning {
            return Ok(());
        }

        if !saw_active_scan && !scanning && wait_start.elapsed() >= startup_grace {
            return Ok(());
        }

        if wait_start.elapsed() >= timeout {
            return Err(ShellError::Generic(GenericError::new(
                "idx scan timed out",
                "timed out waiting for the initial scan to finish (300 s). The index is still available with partial results.",
                span,
            )));
        }

        std::thread::sleep(poll_interval);
    }
}

/// Clone the current runtime handles while holding the global mutex briefly.
fn current_runtime() -> Option<IdxRuntime> {
    let guard = runtime().lock().ok()?;
    guard.as_ref().cloned()
}

/// Get the current runtime, or error if it is not initialized.
fn require_runtime(span: Span) -> Result<IdxRuntime, ShellError> {
    current_runtime().ok_or_else(|| idx_not_initialized_error(span))
}

/// Return the current idx runtime status.
pub fn current_status() -> IdxStatus {
    let Some(runtime) = current_runtime() else {
        return IdxStatus::default();
    };

    let Ok(guard) = runtime.shared_picker.read() else {
        let duration = freeze_scan_duration_if_needed(
            &runtime.scan_completed,
            &runtime.scan_duration_ns,
            runtime.scan_start_time,
            false,
        );
        return IdxStatus {
            initialized: true,
            base_path: runtime.base_path.display().to_string(),
            watch: runtime.watch,
            scanning: false,
            scan_duration_ns: duration,
            ..Default::default()
        };
    };

    guard
        .as_ref()
        .map(|picker| {
            let scanning = picker.is_scan_active();
            let duration = freeze_scan_duration_if_needed(
                &runtime.scan_completed,
                &runtime.scan_duration_ns,
                runtime.scan_start_time,
                scanning,
            );
            idx_status_from_picker(&runtime.base_path, runtime.watch, picker, duration)
        })
        .unwrap_or_default()
}

/// Initialize a live idx runtime backed by `fff-search`.
///
/// When `wait` is true this blocks until the initial filesystem scan finishes.
pub fn init_runtime(
    path: &Path,
    watch: bool,
    wait: bool,
    follow_symlinks: bool,
    enable_content_indexing: bool,
    span: Span,
) -> Result<IdxStatus, ShellError> {
    let shared_picker = SharedFilePicker::default();

    FilePicker::new_with_shared_state(
        shared_picker.clone(),
        SharedFrecency::noop(),
        FilePickerOptions {
            base_path: path.display().to_string(),
            cache_budget: None,
            enable_content_indexing,
            enable_fs_root_scanning: false, // FFF rejects `/` when root scanning is disabled.
            enable_home_dir_scanning: true,
            enable_mmap_cache: false,
            follow_symlinks,
            mode: FFFMode::Ai,
            watch,
        },
    )
    .map_err(|err| fff_error(err, span))?;

    // Store the runtime immediately: background scan and watcher threads are already running, and `idx status` exposes their progress.
    let mut guard = runtime().lock().map_err(|_| {
        ShellError::Generic(GenericError::new(
            "idx runtime lock failed",
            "idx runtime lock poisoned",
            span,
        ))
    })?;

    let previous = guard.replace(IdxRuntime {
        base_path: path.to_path_buf(),
        watch,
        shared_picker: shared_picker.clone(),
        scan_start_time: Instant::now(),
        scan_completed: Arc::new(AtomicBool::new(false)),
        scan_duration_ns: Arc::new(AtomicU64::new(0)),
    });

    // Drop the lock before potentially blocking on --wait so the background scanner can update the shared picker.
    drop(guard);

    // If there was an existing runtime, shut down its watcher cleanly.
    if let Some(old_runtime) = previous {
        let _ = shutdown_shared_picker(&old_runtime.shared_picker, span);
    }

    // Give large repositories up to five minutes to complete when scripts request --wait.
    if wait {
        const WAIT_TIMEOUT: Duration = Duration::from_secs(300);
        wait_for_scan_completion(&shared_picker, WAIT_TIMEOUT, span)?;
        if watch && !shared_picker.wait_for_watcher(WAIT_TIMEOUT) {
            return Err(ShellError::Generic(GenericError::new(
                "idx watcher startup timed out",
                "timed out waiting for the background filesystem watcher to become ready (300 s).",
                span,
            )));
        }
    }
    Ok(current_status())
}

/// Drop the active idx runtime and stop background workers.
pub fn drop_runtime(span: Span) -> Result<Value, ShellError> {
    let mut guard = runtime().lock().map_err(|_| {
        ShellError::Generic(GenericError::new(
            "idx runtime lock failed",
            "idx runtime lock poisoned",
            span,
        ))
    })?;

    let previous_runtime = guard.take();
    drop(guard);

    let dropped = if let Some(runtime) = previous_runtime {
        let _ = shutdown_shared_picker(&runtime.shared_picker, span);
        true
    } else {
        false
    };

    Ok(Value::record(
        record! {
            "dropped" => Value::bool(dropped, span),
            "status" => IdxStatus::default().to_value(span),
        },
        span,
    ))
}

/// Stream indexed directories, optionally filtered by a fuzzy query.
pub fn stream_dirs(
    query: Option<String>,
    span: Span,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let runtime = require_runtime(span)?;
    let shared_picker = runtime.shared_picker;
    let base_path = runtime.base_path;

    let stream: Box<dyn Iterator<Item = Value> + Send> = if let Some(query) = query {
        // FFF returns live item references, so materialize records under the same read lock.
        let values = {
            let guard = read_picker_guard(&shared_picker, span)?;
            let picker = picker_from_guard(&guard, span)?;
            picker
                .fuzzy_search_directories(
                    &QueryParser::default().parse(&query),
                    fuzzy_options(picker.get_dirs().len()),
                )
                .items
                .into_iter()
                .map(|item| build_dir_record(item, picker, &base_path, span))
                .collect::<Vec<_>>()
        };
        Box::new(values.into_iter())
    } else {
        // Surface initialization errors at the command boundary; the lazy iterator can only yield values after this point.
        ensure_picker_initialized(&shared_picker, span)?;
        let mut idx = 0usize;
        Box::new(std::iter::from_fn(move || {
            let guard = match read_picker_guard(&shared_picker, span) {
                Ok(guard) => guard,
                Err(err) => return Some(Value::error(err, span)),
            };
            let picker = match picker_from_guard(&guard, span) {
                Ok(picker) => picker,
                Err(err) => return Some(Value::error(err, span)),
            };

            loop {
                let item = picker.get_dirs().get(idx)?;
                idx = idx.saturating_add(1);
                if !item.is_deleted() {
                    return Some(build_dir_record(item, picker, &base_path, span));
                }
            }
        }))
    };

    Ok(PipelineData::ListStream(
        ListStream::new(stream, span, signals.clone()),
        Some(PipelineMetadata::default()),
    ))
}

/// Stream indexed files, optionally filtered by a fuzzy query.
pub fn stream_files(
    query: Option<String>,
    span: Span,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let runtime = require_runtime(span)?;
    let shared_picker = runtime.shared_picker;
    let base_path = runtime.base_path;

    let stream: Box<dyn Iterator<Item = Value> + Send> = if let Some(query) = query {
        // FFF returns live item references, so materialize records under the same read lock for a coherent point-in-time result without a second lookup pass.
        let values = {
            let guard = read_picker_guard(&shared_picker, span)?;
            let picker = picker_from_guard(&guard, span)?;
            picker
                .fuzzy_search(
                    &QueryParser::default().parse(&query),
                    None,
                    fuzzy_options(picker.get_files().len()),
                )
                .items
                .into_iter()
                .map(|item| build_file_record(item, picker, &base_path, span))
                .collect::<Vec<_>>()
        };
        Box::new(values.into_iter())
    } else {
        // Surface initialization errors at the command boundary; the lazy iterator can only yield values after this point.
        ensure_picker_initialized(&shared_picker, span)?;
        let mut idx = 0usize;
        Box::new(std::iter::from_fn(move || {
            let guard = match read_picker_guard(&shared_picker, span) {
                Ok(guard) => guard,
                Err(err) => return Some(Value::error(err, span)),
            };
            let picker = match picker_from_guard(&guard, span) {
                Ok(picker) => picker,
                Err(err) => return Some(Value::error(err, span)),
            };

            loop {
                let item = picker.get_files().get(idx)?;
                idx = idx.saturating_add(1);
                if !item.is_deleted() {
                    return Some(build_file_record(item, picker, &base_path, span));
                }
            }
        }))
    };

    Ok(PipelineData::ListStream(
        ListStream::new(stream, span, signals.clone()),
        Some(PipelineMetadata::default()),
    ))
}

/// Build a directory record from a live picker's DirItem.
fn build_dir_record(
    item: &fff_search::DirItem,
    picker: &FilePicker,
    base_path: &Path,
    span: Span,
) -> Value {
    let rel_path = item.relative_path(picker);
    let full_path = item.absolute_path(picker, base_path);

    Value::record(
        record! {
            "relative_path" => Value::string(rel_path, span),
            "full_path" => Value::string(full_path.to_string_lossy().into_owned(), span),
        },
        span,
    )
}

/// Build a file record from a live picker's FileItem.
fn build_file_record(
    item: &fff_search::FileItem,
    picker: &FilePicker,
    base_path: &Path,
    span: Span,
) -> Value {
    let file_name = item.file_name(picker);
    let extension = Path::new(&file_name)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default();
    let full_path = item.absolute_path(picker, base_path);
    Value::record(
        record! {
            "relative_path" => Value::string(item.relative_path(picker), span),
            "full_path" => Value::string(full_path.to_string_lossy().into_owned(), span),
            "file_name" => Value::string(file_name, span),
            "ext" => Value::string(extension, span),
            "directory" => Value::string(item.dir_str(picker), span),
            "size" => Value::filesize(i64::try_from(item.size).unwrap_or(i64::MAX), span),
            "modified" => modified_to_date_value(item.modified, span),
        },
        span,
    )
}

/// Convert an indexed file timestamp (unix seconds) to a Nushell date value.
fn modified_to_date_value(modified: u64, span: Span) -> Value {
    let to_fixed = |secs| {
        Utc.timestamp_opt(secs, 0)
            .single()
            .map(|utc| utc.with_timezone(&Local).fixed_offset())
    };

    let secs = i64::try_from(modified).unwrap_or(i64::MAX);
    if let Some(dt) = to_fixed(secs).or_else(|| to_fixed(0)) {
        Value::date(dt, span)
    } else {
        Value::nothing(span)
    }
}

/// Convert a usize to i64, saturating at i64::MAX to avoid overflow.
fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn fuzzy_options(limit: usize) -> FuzzySearchOptions<'static> {
    FuzzySearchOptions {
        max_threads: 0,
        current_file: None,
        project_path: None,
        combo_boost_score_multiplier: 0,
        min_combo_count: 0,
        pagination: PaginationArgs { offset: 0, limit },
    }
}

pub struct FindSearchContext<'a> {
    pub query: &'a str,
    pub files_only: bool,
    pub dirs_only: bool,
    pub verbose: bool,
    pub limit: usize,
    pub span: Span,
    pub cwd: Option<&'a Path>,
    pub signals: &'a Signals,
}

/// Run a fuzzy find across indexed files and/or directories.
pub fn stream_find(context: FindSearchContext<'_>) -> Result<PipelineData, ShellError> {
    let runtime = require_runtime(context.span)?;
    let guard = read_picker_guard(&runtime.shared_picker, context.span)?;
    let picker = picker_from_guard(&guard, context.span)?;

    let parser = QueryParser::default();
    let parsed = parser.parse(context.query);
    let options = fuzzy_options(context.limit);

    let find_data: Vec<Value> = if !context.files_only && !context.dirs_only {
        let result = picker.fuzzy_search_mixed(&parsed, None, options);

        result
            .items
            .into_iter()
            .zip(result.scores)
            .enumerate()
            .map(|(rank, (item, score))| {
                let (kind, path) = match item {
                    MixedItemRef::File(file) => (
                        "file",
                        file_path_for_cwd(file, picker, &runtime.base_path, context.cwd),
                    ),
                    MixedItemRef::Dir(dir) => (
                        "dir",
                        dir_path_for_cwd(dir, picker, &runtime.base_path, context.cwd),
                    ),
                };

                let mut record = record! {
                    "kind" => Value::string(kind, context.span),
                    "relative_path" => Value::string(path, context.span),
                    "rank" => Value::int(usize_to_i64(rank + 1), context.span),
                    "score" => Value::int(i64::from(score.total), context.span),
                };

                if context.verbose {
                    record.push(
                        "score_details",
                        Value::record(
                            record! {
                                "base_score" => Value::int(i64::from(score.base_score), context.span),
                                "filename_bonus" => Value::int(i64::from(score.filename_bonus), context.span),
                                "special_filename_bonus" => Value::int(i64::from(score.special_filename_bonus), context.span),
                                "frecency_boost" => Value::int(i64::from(score.frecency_boost), context.span),
                            },
                            context.span,
                        ),
                    );
                }

                Value::record(record, context.span)
            })
            .collect()
    } else if context.dirs_only {
        let result = picker.fuzzy_search_directories(&parsed, options);
        result
            .items
            .into_iter()
            .zip(result.scores)
            .enumerate()
            .map(|(rank, (item, score))| {
                let mut record = record! {
                    "kind" => Value::string("dir", context.span),
                    "relative_path" => Value::string(dir_path_for_cwd(item, picker, &runtime.base_path, context.cwd), context.span),
                    "rank" => Value::int(usize_to_i64(rank + 1), context.span),
                    "score" => Value::int(i64::from(score.total), context.span),
                };

                if context.verbose {
                    record.push("exact_match", Value::bool(score.exact_match, context.span));
                }

                Value::record(record, context.span)
            })
            .collect()
    } else {
        let result = picker.fuzzy_search(&parsed, None, options);
        result
            .items
            .into_iter()
            .zip(result.scores)
            .enumerate()
            .map(|(rank, (item, score))| {
                let mut record = record! {
                    "kind" => Value::string("file", context.span),
                    "relative_path" => Value::string(file_path_for_cwd(item, picker, &runtime.base_path, context.cwd), context.span),
                    "rank" => Value::int(usize_to_i64(rank + 1), context.span),
                    "score" => Value::int(i64::from(score.total), context.span),
                };

                if context.verbose {
                    record.push("match_type", Value::string(score.match_type, context.span));
                }

                Value::record(record, context.span)
            })
            .collect()
    };

    drop(guard);

    Ok(PipelineData::ListStream(
        ListStream::new(find_data.into_iter(), context.span, context.signals.clone()),
        Some(PipelineMetadata::default()),
    ))
}

pub struct GrepSearchContext<'a> {
    pub patterns: &'a [String],
    pub mode: GrepMode,
    pub page_limit: usize,
    pub span: Span,
    pub before_context: usize,
    pub after_context: usize,
    pub cwd: Option<&'a Path>,
    pub signals: &'a Signals,
}

/// Search indexed file contents (`idx search`).
pub fn stream_grep(context: GrepSearchContext<'_>) -> Result<PipelineData, ShellError> {
    let runtime = require_runtime(context.span)?;

    let guard = read_picker_guard(&runtime.shared_picker, context.span)?;
    let picker = picker_from_guard(&guard, context.span)?;

    let options = GrepSearchOptions {
        mode: context.mode,
        page_limit: context.page_limit,
        before_context: context.before_context,
        after_context: context.after_context,
        ..Default::default()
    };

    // GrepConfig treats `[`, `?`, and bare `*` as searchable source text unless they form an explicit path or brace glob.
    let parser = QueryParser::new(GrepConfig);

    let result = if context.patterns.len() == 1 {
        let query = parser.parse(&context.patterns[0]);
        picker.grep(&query, &options)
    } else {
        // Extract file constraints from every pattern while preserving the remaining text for multi-pattern matching.
        let mut all_constraints = Vec::new();
        let mut text_patterns: Vec<String> = Vec::new();
        for pat in context.patterns {
            let query = parser.parse(pat.as_str());
            let text = query.grep_text();
            all_constraints.extend(query.constraints);
            if !text.is_empty() {
                text_patterns.push(text);
            }
        }
        let refs: Vec<&str> = text_patterns.iter().map(String::as_str).collect();
        picker.multi_grep(&refs, &all_constraints, &options)
    };

    let file_paths = result
        .files
        .iter()
        .map(|file| file_path_for_cwd(file, picker, &runtime.base_path, context.cwd))
        .collect::<Vec<_>>();
    let matches = result.matches;

    drop(guard);

    let stream = matches.into_iter().map(move |item| {
        // FFF remaps every GrepMatch::file_index into the returned deduplicated files vector.
        debug_assert!(item.file_index < file_paths.len());
        let relative_path = file_paths[item.file_index].clone();
        let byte_offset = i64::try_from(item.byte_offset).unwrap_or(i64::MAX);

        let offsets = item
            .match_byte_offsets
            .iter()
            .map(|(start, end)| {
                Value::record(
                    record! {
                        "start" => Value::int(byte_offset.saturating_add(i64::from(*start)), context.span),
                        "end" => Value::int(byte_offset.saturating_add(i64::from(*end)), context.span),
                    },
                    context.span,
                )
            })
            .collect::<Vec<_>>();

        let context_lines = (!item.context_before.is_empty() || !item.context_after.is_empty())
            .then(|| {
                item
                .context_before
                .iter()
                .map(|l| Value::string(l.clone(), context.span))
                .chain(std::iter::once(Value::string(
                    item.line_content.clone(),
                    context.span,
                )))
                .chain(
                    item.context_after
                        .iter()
                        .map(|l| Value::string(l.clone(), context.span)),
                )
                .collect::<Vec<_>>()
            });

        let mut record = record! {
            "relative_path" => Value::string(relative_path, context.span),
            "line_number" => Value::int(i64::try_from(item.line_number).unwrap_or(i64::MAX), context.span),
            "column" => Value::int(usize_to_i64(item.col), context.span),
            "byte_offset" => Value::int(byte_offset, context.span),
            "line" => Value::string(item.line_content, context.span),
            "matches" => Value::list(offsets, context.span),
        };

        if let Some(context_lines) = context_lines {
            record.push("with_context", Value::list(context_lines, context.span));
        }

        Value::record(record, context.span)
    });

    Ok(PipelineData::ListStream(
        ListStream::new(stream, context.span, context.signals.clone()),
        Some(PipelineMetadata::default()),
    ))
}

/// How often the watch stream polls for Ctrl-C / timeout while waiting on events.
const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(100);
/// How long `idx watch` waits for the background OS watcher after a non-blocking init.
const WATCHER_READY_TIMEOUT: Duration = Duration::from_secs(30);

/// Options for [`stream_watch`].
pub struct WatchStreamOptions {
    /// Base-relative glob, exact path, directory, or empty for the whole tree.
    pub pattern: String,
    /// Additional glob or path-prefix exclusions (fff `WatchOptions.ignore`).
    pub ignore: Vec<String>,
    /// Optional duration after which the stream ends cleanly.
    pub timeout: Option<Duration>,
    /// Optional cap on the number of events emitted before the stream ends.
    pub max_events: Option<usize>,
    pub span: Span,
    pub signals: Signals,
}

/// Subscribe to filesystem changes on the live idx runtime and stream events as records.
///
/// Requires an initialized runtime with watching enabled. Each event is a record `{ kind, path }`.
/// Batches from fff-search are flattened to one pipeline item each.
pub fn stream_watch(options: WatchStreamOptions) -> Result<PipelineData, ShellError> {
    let runtime = require_runtime(options.span)?;

    if !runtime.watch {
        return Err(ShellError::Generic(GenericError::new(
            "idx watching is disabled",
            "re-run `idx init` without `--no-watch` to enable filesystem watching",
            options.span,
        )));
    }

    if !runtime
        .shared_picker
        .wait_for_watcher(WATCHER_READY_TIMEOUT)
    {
        return Err(ShellError::Generic(GenericError::new(
            "idx watcher not ready",
            "timed out waiting for the background filesystem watcher to become ready (30 s). Try `idx init <path> --wait` first.",
            options.span,
        )));
    }

    let (tx, rx): (Sender<Vec<WatchEvent>>, Receiver<Vec<WatchEvent>>) = channel();
    let shared_picker = runtime.shared_picker;

    let watch_id = shared_picker
        .watch(
            &options.pattern,
            WatchOptions {
                ignore: options.ignore,
            },
            move |_id, events| {
                // Best-effort: if the receiver is gone the stream already ended.
                let _ = tx.send(events.to_vec());
            },
        )
        .map_err(|err| fff_error(err, options.span))?;

    let stream = WatchEventStream {
        rx: Some(rx),
        pending: Vec::new().into_iter(),
        events_emitted: 0,
        max_events: options.max_events,
        deadline: options.timeout.map(|d| Instant::now() + d),
        signals: options.signals.clone(),
        span: options.span,
        cleanup: Some(WatchSubscription {
            shared_picker,
            watch_id,
        }),
    };

    Ok(PipelineData::ListStream(
        ListStream::new(stream, options.span, options.signals),
        Some(PipelineMetadata::default()),
    ))
}

/// RAII guard that unsubscribes a fff-search watch when the stream ends.
struct WatchSubscription {
    shared_picker: SharedFilePicker,
    watch_id: WatchId,
}

impl Drop for WatchSubscription {
    fn drop(&mut self) {
        let _ = self.shared_picker.unwatch(self.watch_id);
    }
}

/// Blocking iterator that flattens debounced watch batches into Nushell records.
struct WatchEventStream {
    rx: Option<Receiver<Vec<WatchEvent>>>,
    pending: std::vec::IntoIter<WatchEvent>,
    events_emitted: usize,
    max_events: Option<usize>,
    deadline: Option<Instant>,
    signals: Signals,
    span: Span,
    cleanup: Option<WatchSubscription>,
}

impl WatchEventStream {
    fn finish(&mut self) {
        // Drop the receiver first so the callback channel closes, then unwatch.
        self.rx = None;
        self.cleanup.take();
    }

    fn next_pending(&mut self) -> Option<Value> {
        let event = self.pending.next()?;
        self.events_emitted += 1;
        Some(watch_event_to_value(event, self.span))
    }
}

impl Iterator for WatchEventStream {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self
                .max_events
                .is_some_and(|max| self.events_emitted >= max)
                || self.deadline.is_some_and(|d| Instant::now() >= d)
            {
                self.finish();
                return None;
            }

            // ListStream checks signals before calling next(), but this iterator can block inside recv_timeout, so it
            // must also poll between waits.
            if let Err(err) = self.signals.check(&self.span) {
                self.finish();
                return Some(Value::error(err, self.span));
            }

            // Emit the last allowed event; the next call observes max_events and tears the subscription down.
            if let Some(value) = self.next_pending() {
                return Some(value);
            }

            let rx = self.rx.as_ref()?;
            match rx.recv_timeout(WATCH_POLL_INTERVAL) {
                Ok(batch) => self.pending = batch.into_iter(),
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    self.finish();
                    return None;
                }
            }
        }
    }
}

fn watch_event_to_value(event: WatchEvent, span: Span) -> Value {
    // Paths are absolute (fff WatchEvent); Nu represents filesystem paths as strings.
    Value::record(
        record! {
            "kind" => Value::string(event.kind.as_str(), span),
            "path" => Value::string(event.path.to_string_lossy(), span),
        },
        span,
    )
}

fn path_for_cwd(path: &Path, cwd: Option<&Path>, fallback: &str) -> String {
    let Some(cwd) = cwd else {
        return fallback.to_string();
    };

    diff_paths(path, cwd)
        .map(|path| {
            if path.as_os_str().is_empty() {
                ".".to_string()
            } else {
                path.to_string_lossy().into_owned()
            }
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn file_path_for_cwd(
    file: &fff_search::FileItem,
    picker: &FilePicker,
    base_path: &Path,
    cwd: Option<&Path>,
) -> String {
    let absolute_path = file.absolute_path(picker, base_path);
    path_for_cwd(&absolute_path, cwd, &file.relative_path(picker))
}

fn dir_path_for_cwd(
    dir: &fff_search::DirItem,
    picker: &FilePicker,
    base_path: &Path,
    cwd: Option<&Path>,
) -> String {
    let absolute_path = dir.absolute_path(picker, base_path);
    path_for_cwd(&absolute_path, cwd, &dir.relative_path(picker))
}
