//! In-process file indexing runtime for the `idx` command family.
//!
//! This module manages two types of index backends:
//! - **Live mode** (after `idx init`): Backed by a `fff-search` FilePicker that scans and watches the filesystem
//! - **Snapshot mode** (after `idx import`): Backed by pre-computed file/directory metadata restored from a snapshot
//!
//! The runtime is stored as a thread-safe singleton (`IDX_RUNTIME`) and can be accessed by all idx subcommands.
//! Public functions handle initialization, status reporting, and streaming results to Nushell.

use chrono::{DateTime, Local, LocalResult, TimeZone, Utc};
use fff_search::{
    FFFMode, FilePicker, FilePickerOptions, FuzzySearchOptions, GrepMode, GrepSearchOptions,
    MixedItemRef, PaginationArgs, QueryParser, SharedFilePicker, SharedFrecency,
};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{ListStream, PipelineMetadata, Signals};
use nu_utils::time::Instant;
#[cfg(feature = "sqlite")]
use rusqlite::{Connection, OptionalExtension, params};
#[cfg(feature = "sqlite")]
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Global in-process runtime for idx commands.
///
/// The runtime is shared by all idx subcommands and can be backed either by
/// a live `fff-search` picker (after `idx init`) or by restored snapshot rows
/// (after `idx import`).
pub struct IdxRuntime {
    pub base_path: PathBuf,
    pub watch: bool,
    pub shared_picker: SharedFilePicker,
    pub scan_start_time: Instant,
    pub scan_completed: Arc<AtomicBool>,
    pub scan_duration_ns: Arc<AtomicU64>,
    pub restored_files: Option<Arc<Vec<IdxRestoredFile>>>,
    pub restored_dirs: Option<Arc<Vec<IdxRestoredDir>>>,
    pub restored_arena_bytes_base: usize,
    pub restored_arena_bytes_overflow: usize,
}

#[derive(Clone, Debug)]
/// Snapshot-backed file row used by imported idx runtimes.
///
/// Fields are restored from the SQLite snapshot database and are used for
/// fuzzy searching when the original filesystem may no longer be available.
pub struct IdxRestoredFile {
    /// Relative path from the index base directory.
    pub relative_path: String,
    /// Absolute path to the file (may not exist on disk).
    pub full_path: String,
    /// Just the filename component without directory path.
    pub file_name: String,
    /// Parent directory path as a string.
    pub directory: String,
    /// File size in bytes.
    pub size: u64,
    /// Last modified timestamp as seconds since epoch.
    pub modified: u64,
}

#[derive(Clone, Debug)]
/// Snapshot-backed directory row used by imported idx runtimes.
///
/// Fields are restored from the SQLite snapshot database for use in
/// fuzzy directory searches when the filesystem may not be available.
pub struct IdxRestoredDir {
    /// Relative path from the index base directory.
    pub relative_path: String,
    /// Absolute path to the directory (may not exist on disk).
    pub full_path: String,
}

/// A cloned snapshot of the current runtime state, obtained while holding the
/// global mutex for the minimum amount of time. All fields are cheaply
/// cloneable (Arc / Copy), so callers can work with this after the lock is
/// released.
pub type RuntimeSnapshot = IdxRuntime;

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
        Value::record(
            [
                (
                    "initialized".to_string(),
                    Value::bool(self.initialized, span),
                ),
                (
                    "base_path".to_string(),
                    Value::string(self.base_path.clone(), span),
                ),
                ("watch".to_string(), Value::bool(self.watch, span)),
                ("scanning".to_string(), Value::bool(self.scanning, span)),
                (
                    "scan_duration".to_string(),
                    Value::duration(
                        i64::try_from(self.scan_duration_ns).unwrap_or(i64::MAX),
                        span,
                    ),
                ),
                (
                    "files".to_string(),
                    Value::int(i64::try_from(self.files).unwrap_or(i64::MAX), span),
                ),
                (
                    "dirs".to_string(),
                    Value::int(i64::try_from(self.dirs).unwrap_or(i64::MAX), span),
                ),
                (
                    "arena_size_base".to_string(),
                    Value::filesize(
                        i64::try_from(self.arena_bytes_base).unwrap_or(i64::MAX),
                        span,
                    ),
                ),
                (
                    "arena_size_overflow".to_string(),
                    Value::filesize(
                        i64::try_from(self.arena_bytes_overflow).unwrap_or(i64::MAX),
                        span,
                    ),
                ),
                (
                    "arena_size_untracked".to_string(),
                    Value::filesize(
                        i64::try_from(self.arena_bytes_untracked).unwrap_or(i64::MAX),
                        span,
                    ),
                ),
                (
                    "arena_size_total".to_string(),
                    Value::filesize(
                        i64::try_from(
                            self.arena_bytes_base
                                .saturating_add(self.arena_bytes_overflow)
                                .saturating_add(self.arena_bytes_untracked),
                        )
                        .unwrap_or(i64::MAX),
                        span,
                    ),
                ),
            ]
            .into_iter()
            .collect(),
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

/// Validate that the picker has been initialized.
///
/// Returns an error if the runtime is not yet initialized or if the shared
/// picker lock is poisoned.
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
/// Never hold the shared picker write lock while joining the watcher thread,
/// otherwise the owner thread can deadlock waiting for the same lock while
/// processing a final event batch.
fn shutdown_shared_picker(shared_picker: &SharedFilePicker, span: Span) -> Result<(), ShellError> {
    // Important: never join the watcher thread while holding the shared
    // picker write lock, otherwise the owner thread can deadlock waiting for
    // the same lock while processing a final event batch.
    let mut picker_to_stop = {
        let mut guard = shared_picker.write().map_err(|err| fff_error(err, span))?;
        guard.take()
    };

    if let Some(picker) = picker_to_stop.as_mut() {
        // First tell background workers to stop taking new work, then
        // synchronously stop/join the watcher owner thread.
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
    IdxStatus {
        initialized: true,
        base_path: base_path.display().to_string(),
        watch,
        scanning: picker.is_scan_active(),
        scan_duration_ns,
        files: picker.get_files().len(),
        dirs: picker.get_dirs().len(),
        arena_bytes_base: picker.arena_bytes().0,
        arena_bytes_overflow: picker.arena_bytes().1,
        arena_bytes_untracked: picker.arena_bytes().2,
    }
}

/// Get elapsed nanoseconds since the given scan start time.
fn elapsed_ns(scan_start: Instant) -> u64 {
    u64::try_from(scan_start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// Compute the scan duration, freezing it once the scan completes.
///
/// This uses atomic operations to ensure the completion time is recorded exactly once,
/// even if called concurrently from multiple threads.
fn freeze_scan_duration_if_needed(
    scan_completed: &AtomicBool,
    scan_duration_ns: &AtomicU64,
    scan_start: Instant,
    scanning: bool,
) -> u64 {
    if scan_completed.load(Ordering::Acquire) {
        return scan_duration_ns.load(Ordering::Acquire);
    }

    let elapsed = elapsed_ns(scan_start);
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

/// Retrieve a thread-safe snapshot of the current idx runtime state.
///
/// This is a cheap operation that clones only Arc/Copy data, so callers can
/// work with the snapshot after releasing the global lock.
fn runtime_snapshot() -> Option<RuntimeSnapshot> {
    let guard = runtime().lock().ok()?;
    let runtime = guard.as_ref()?;
    Some(RuntimeSnapshot {
        base_path: runtime.base_path.clone(),
        watch: runtime.watch,
        shared_picker: runtime.shared_picker.clone(),
        scan_start_time: runtime.scan_start_time,
        scan_completed: runtime.scan_completed.clone(),
        scan_duration_ns: runtime.scan_duration_ns.clone(),
        restored_files: runtime.restored_files.clone(),
        restored_dirs: runtime.restored_dirs.clone(),
        restored_arena_bytes_base: runtime.restored_arena_bytes_base,
        restored_arena_bytes_overflow: runtime.restored_arena_bytes_overflow,
    })
}

/// Get a runtime snapshot, or error if the runtime is not initialized.
fn require_runtime(span: Span) -> Result<RuntimeSnapshot, ShellError> {
    runtime_snapshot().ok_or_else(|| idx_not_initialized_error(span))
}

/// Return the current idx runtime status.
///
/// `scan_start_override` exists for callers that need deterministic status
/// snapshots while coordinating scan lifecycle checks.
pub fn current_status(scan_start_override: Option<Instant>) -> IdxStatus {
    let Some(snapshot) = runtime_snapshot() else {
        return IdxStatus::default();
    };

    if let (Some(restored_files), Some(restored_dirs)) = (
        snapshot.restored_files.as_ref(),
        snapshot.restored_dirs.as_ref(),
    ) {
        return IdxStatus {
            initialized: true,
            base_path: snapshot.base_path.display().to_string(),
            watch: snapshot.watch,
            scanning: false,
            scan_duration_ns: 0,
            files: restored_files.len(),
            dirs: restored_dirs.len(),
            arena_bytes_base: snapshot.restored_arena_bytes_base,
            arena_bytes_overflow: snapshot.restored_arena_bytes_overflow,
            arena_bytes_untracked: 0,
        };
    }

    let scan_start = scan_start_override.unwrap_or(snapshot.scan_start_time);

    let Ok(guard) = snapshot.shared_picker.read() else {
        let duration = freeze_scan_duration_if_needed(
            &snapshot.scan_completed,
            &snapshot.scan_duration_ns,
            scan_start,
            false,
        );
        return IdxStatus {
            initialized: true,
            base_path: snapshot.base_path.display().to_string(),
            watch: snapshot.watch,
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
                &snapshot.scan_completed,
                &snapshot.scan_duration_ns,
                scan_start,
                scanning,
            );
            idx_status_from_picker(&snapshot.base_path, snapshot.watch, picker, duration)
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
    let shared_frecency = SharedFrecency::noop();

    FilePicker::new_with_shared_state(
        shared_picker.clone(),
        shared_frecency,
        FilePickerOptions {
            base_path: path.display().to_string(),
            cache_budget: None,
            enable_content_indexing,
            enable_fs_root_scanning: false, // this will error out if executed at /
            enable_home_dir_scanning: true,
            enable_mmap_cache: false,
            follow_symlinks,
            mode: FFFMode::Ai,
            watch,
        },
    )
    .map_err(|err| fff_error(err, span))?;

    // Store the runtime immediately — background scan and watcher threads are
    // already running. Callers can check `idx status` to see when scanning
    // completes (`scanning: false`).
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
        restored_files: None,
        restored_dirs: None,
        restored_arena_bytes_base: 0,
        restored_arena_bytes_overflow: 0,
    });

    // Drop the lock before potentially blocking on --wait so other threads
    // (e.g. the background scanner writing into the shared picker) are not
    // deadlocked against us.
    drop(guard);

    // If there was an existing runtime, shut down its watcher cleanly.
    if let Some(old_runtime) = previous {
        let _ = shutdown_shared_picker(&old_runtime.shared_picker, span);
    }

    // --wait: block until the background scan finishes (useful for scripts).
    // We give the scan up to 5 minutes before giving up; this is intentionally
    // generous so that large repos complete without errors.
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
    Ok(current_status(None))
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

    if let Some(runtime) = previous_runtime.as_ref() {
        let _ = shutdown_shared_picker(&runtime.shared_picker, span);
    }

    let dropped = previous_runtime.is_some();

    Ok(Value::record(
        [
            ("dropped".to_string(), Value::bool(dropped, span)),
            ("status".to_string(), IdxStatus::default().to_value(span)),
        ]
        .into_iter()
        .collect(),
        span,
    ))
}

/// Stream indexed directories, optionally filtered by a fuzzy query.
///
/// Uses snapshot-backed rows when runtime comes from `idx import`, otherwise
/// reads from the live picker.
pub fn stream_dirs(
    query: Option<String>,
    span: Span,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let snapshot = require_runtime(span)?;

    if let Some(restored_dirs) = snapshot.restored_dirs.clone() {
        let ranked_indices = query
            .as_deref()
            .map(|q| rank_snapshot_matches(&restored_dirs, q, |dir| &dir.relative_path));

        let stream_signals = signals.clone();
        let stream: Box<dyn Iterator<Item = Value> + Send> = match ranked_indices {
            Some(indices) => {
                let mut iter = indices.into_iter();
                let restored_dirs = restored_dirs.clone();
                Box::new(std::iter::from_fn(move || {
                    if let Err(err) = stream_signals.check(&span) {
                        return Some(Value::error(err, span));
                    }

                    let idx = iter.next()?;
                    restored_dirs
                        .get(idx)
                        .map(|item| build_restored_dir_record(item, span))
                }))
            }
            None => {
                let mut idx = 0usize;
                let restored_dirs = restored_dirs.clone();
                Box::new(std::iter::from_fn(move || {
                    if let Err(err) = stream_signals.check(&span) {
                        return Some(Value::error(err, span));
                    }

                    let item = restored_dirs.get(idx)?;
                    idx = idx.saturating_add(1);
                    Some(build_restored_dir_record(item, span))
                }))
            }
        };

        return Ok(PipelineData::ListStream(
            ListStream::new(stream, span, signals.clone()),
            Some(PipelineMetadata::default()),
        ));
    }

    let shared_picker = snapshot.shared_picker.clone();
    let base_path = snapshot.base_path.clone();

    // Validate runtime before constructing the lazy iterator.
    ensure_picker_initialized(&shared_picker, span)?;

    let stream: Box<dyn Iterator<Item = Value> + Send> = if let Some(query) = query {
        let shared_picker_for_query = shared_picker.clone();
        let matched_paths = {
            let guard = read_picker_guard(&shared_picker_for_query, span)?;
            let picker = picker_from_guard(&guard, span)?;

            let parser = QueryParser::default();
            let parsed = parser.parse(&query);
            let options = FuzzySearchOptions {
                max_threads: 0,
                current_file: None,
                project_path: None,
                combo_boost_score_multiplier: 0,
                min_combo_count: 0,
                pagination: PaginationArgs {
                    offset: 0,
                    limit: picker.get_dirs().len(),
                },
            };

            picker
                .fuzzy_search_directories(&parsed, options)
                .items
                .iter()
                .map(|item| item.relative_path(picker))
                .collect::<Vec<_>>()
        };

        let mut path_iter = matched_paths.into_iter();
        let stream_signals = signals.clone();
        let shared_picker = shared_picker.clone();
        Box::new(std::iter::from_fn(move || {
            loop {
                if let Err(err) = stream_signals.check(&span) {
                    return Some(Value::error(err, span));
                }

                let path = path_iter.next()?;

                let guard = match read_picker_guard(&shared_picker, span) {
                    Ok(guard) => guard,
                    Err(err) => return Some(Value::error(err, span)),
                };
                let picker = match picker_from_guard(&guard, span) {
                    Ok(picker) => picker,
                    Err(err) => return Some(Value::error(err, span)),
                };

                let item = picker
                    .get_dirs()
                    .iter()
                    .find(|item| item.relative_path(picker) == path);
                if let Some(item) = item {
                    return Some(build_dir_record(item, picker, &base_path, span));
                }
            }
        }))
    } else {
        let mut idx = 0usize;
        let stream_signals = signals.clone();
        let shared_picker = shared_picker.clone();
        Box::new(std::iter::from_fn(move || {
            if let Err(err) = stream_signals.check(&span) {
                return Some(Value::error(err, span));
            }

            let guard = match read_picker_guard(&shared_picker, span) {
                Ok(guard) => guard,
                Err(err) => return Some(Value::error(err, span)),
            };
            let picker = match picker_from_guard(&guard, span) {
                Ok(picker) => picker,
                Err(err) => return Some(Value::error(err, span)),
            };

            let item = picker.get_dirs().get(idx)?;
            idx = idx.saturating_add(1);
            Some(build_dir_record(item, picker, &base_path, span))
        }))
    };

    Ok(PipelineData::ListStream(
        ListStream::new(stream, span, signals.clone()),
        Some(PipelineMetadata::default()),
    ))
}

/// Stream indexed files, optionally filtered by a fuzzy query.
///
/// Uses snapshot-backed rows when runtime comes from `idx import`, otherwise
/// reads from the live picker.
pub fn stream_files(
    query: Option<String>,
    span: Span,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let snapshot = require_runtime(span)?;

    if let Some(restored_files) = snapshot.restored_files.clone() {
        let ranked_indices = query
            .as_deref()
            .map(|q| rank_snapshot_matches(&restored_files, q, |file| &file.relative_path));

        let stream_signals = signals.clone();
        let stream: Box<dyn Iterator<Item = Value> + Send> = match ranked_indices {
            Some(indices) => {
                let mut iter = indices.into_iter();
                let restored_files = restored_files.clone();
                Box::new(std::iter::from_fn(move || {
                    if let Err(err) = stream_signals.check(&span) {
                        return Some(Value::error(err, span));
                    }

                    let idx = iter.next()?;
                    restored_files
                        .get(idx)
                        .map(|item| build_restored_file_record(item, span))
                }))
            }
            None => {
                let mut idx = 0usize;
                let restored_files = restored_files.clone();
                Box::new(std::iter::from_fn(move || {
                    if let Err(err) = stream_signals.check(&span) {
                        return Some(Value::error(err, span));
                    }

                    let item = restored_files.get(idx)?;
                    idx = idx.saturating_add(1);
                    Some(build_restored_file_record(item, span))
                }))
            }
        };

        return Ok(PipelineData::ListStream(
            ListStream::new(stream, span, signals.clone()),
            Some(PipelineMetadata::default()),
        ));
    }

    let shared_picker = snapshot.shared_picker.clone();
    let base_path = snapshot.base_path.clone();

    // Validate runtime before constructing the lazy iterator.
    ensure_picker_initialized(&shared_picker, span)?;

    let stream: Box<dyn Iterator<Item = Value> + Send> = if let Some(query) = query {
        let shared_picker_for_query = shared_picker.clone();
        let matched_paths = {
            let guard = read_picker_guard(&shared_picker_for_query, span)?;
            let picker = picker_from_guard(&guard, span)?;

            let parser = QueryParser::default();
            let parsed = parser.parse(&query);
            let options = FuzzySearchOptions {
                max_threads: 0,
                current_file: None,
                project_path: None,
                combo_boost_score_multiplier: 0,
                min_combo_count: 0,
                pagination: PaginationArgs {
                    offset: 0,
                    limit: picker.get_files().len(),
                },
            };

            picker
                .fuzzy_search(&parsed, None, options)
                .items
                .iter()
                .map(|item| item.relative_path(picker))
                .collect::<Vec<_>>()
        };

        let mut path_iter = matched_paths.into_iter();
        let stream_signals = signals.clone();
        let shared_picker = shared_picker.clone();
        Box::new(std::iter::from_fn(move || {
            loop {
                if let Err(err) = stream_signals.check(&span) {
                    return Some(Value::error(err, span));
                }

                let path = path_iter.next()?;

                let guard = match read_picker_guard(&shared_picker, span) {
                    Ok(guard) => guard,
                    Err(err) => return Some(Value::error(err, span)),
                };
                let picker = match picker_from_guard(&guard, span) {
                    Ok(picker) => picker,
                    Err(err) => return Some(Value::error(err, span)),
                };

                let item = picker
                    .get_files()
                    .iter()
                    .find(|item| item.relative_path(picker) == path);
                if let Some(item) = item {
                    return Some(build_file_record(item, picker, &base_path, span));
                }
            }
        }))
    } else {
        let mut idx = 0usize;
        let stream_signals = signals.clone();
        let shared_picker = shared_picker.clone();
        Box::new(std::iter::from_fn(move || {
            if let Err(err) = stream_signals.check(&span) {
                return Some(Value::error(err, span));
            }

            let guard = match read_picker_guard(&shared_picker, span) {
                Ok(guard) => guard,
                Err(err) => return Some(Value::error(err, span)),
            };
            let picker = match picker_from_guard(&guard, span) {
                Ok(picker) => picker,
                Err(err) => return Some(Value::error(err, span)),
            };

            let item = picker.get_files().get(idx)?;
            idx = idx.saturating_add(1);
            Some(build_file_record(item, picker, &base_path, span))
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

    build_record_from_cols(
        [
            ("relative_path".to_string(), Value::string(rel_path, span)),
            (
                "full_path".to_string(),
                Value::string(full_path.to_string_lossy().to_string(), span),
            ),
        ],
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
    let full_path = item.absolute_path(picker, base_path);
    build_record_from_cols(
        [
            (
                "relative_path".to_string(),
                Value::string(item.relative_path(picker), span),
            ),
            (
                "full_path".to_string(),
                Value::string(full_path.to_string_lossy().to_string(), span),
            ),
            (
                "file_name".to_string(),
                Value::string(file_name.clone(), span),
            ),
            (
                "ext".to_string(),
                Value::string(file_extension(&file_name), span),
            ),
            (
                "directory".to_string(),
                Value::string(item.dir_str(picker), span),
            ),
            (
                "size".to_string(),
                Value::filesize(i64::try_from(item.size).unwrap_or(i64::MAX), span),
            ),
            (
                "modified".to_string(),
                modified_to_date_value(item.modified, span),
            ),
        ],
        span,
    )
}

/// Build a directory record from a restored snapshot.
fn build_restored_dir_record(item: &IdxRestoredDir, span: Span) -> Value {
    build_record_from_cols(
        [
            (
                "relative_path".to_string(),
                Value::string(item.relative_path.clone(), span),
            ),
            (
                "full_path".to_string(),
                Value::string(item.full_path.clone(), span),
            ),
        ],
        span,
    )
}

/// Build a file record from a restored snapshot.
fn build_restored_file_record(item: &IdxRestoredFile, span: Span) -> Value {
    build_record_from_cols(
        [
            (
                "relative_path".to_string(),
                Value::string(item.relative_path.clone(), span),
            ),
            (
                "full_path".to_string(),
                Value::string(item.full_path.clone(), span),
            ),
            (
                "file_name".to_string(),
                Value::string(item.file_name.clone(), span),
            ),
            (
                "ext".to_string(),
                Value::string(file_extension(&item.file_name), span),
            ),
            (
                "directory".to_string(),
                Value::string(item.directory.clone(), span),
            ),
            (
                "size".to_string(),
                Value::filesize(i64::try_from(item.size).unwrap_or(i64::MAX), span),
            ),
            (
                "modified".to_string(),
                modified_to_date_value(item.modified, span),
            ),
        ],
        span,
    )
}

/// Convert an indexed file timestamp (unix seconds) to a Nushell date value.
fn modified_to_date_value(modified: u64, span: Span) -> Value {
    let to_fixed = |secs: i64| -> Option<DateTime<chrono::FixedOffset>> {
        let utc = match Utc.timestamp_opt(secs, 0) {
            LocalResult::Single(ts) => ts,
            _ => return None,
        };
        let local = utc.with_timezone(&Local);
        Some(local.with_timezone(local.offset()))
    };

    let secs = i64::try_from(modified).unwrap_or(i64::MAX);
    if let Some(dt) = to_fixed(secs).or_else(|| to_fixed(0)) {
        Value::date(dt, span)
    } else {
        Value::nothing(span)
    }
}

/// Extract a file extension without the leading dot.
fn file_extension(file_name: &str) -> String {
    Path::new(file_name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Helper to construct a Nushell record from a small array of columns.
#[inline]
fn build_record_from_cols<const N: usize>(cols: [(String, Value); N], span: Span) -> Value {
    Value::record(cols.into_iter().collect(), span)
}

/// Score how well a path matches a query using fuzzy matching heuristics.
///
/// Returns `None` if no match found, `Some(score)` where higher scores are better matches.
/// Scoring prioritizes exact matches, prefix matches, substring matches, and finally
/// fuzzy character sequences.
fn score_snapshot_match(path: &str, query: &str) -> Option<i64> {
    let query = query.trim();
    if query.is_empty() {
        return Some(0);
    }

    let path_lower = path.to_ascii_lowercase();
    let query_lower = query.to_ascii_lowercase();

    if path_lower == query_lower {
        return Some(4_000);
    }

    if path_lower.starts_with(&query_lower) {
        return Some(3_000);
    }

    if let Some(pos) = path_lower.find(&query_lower) {
        let proximity = i64::try_from(pos).unwrap_or(i64::MAX);
        return Some(2_000_i64.saturating_sub(proximity));
    }

    let mut query_iter = query_lower.chars();
    let mut needle = query_iter.next()?;
    for ch in path_lower.chars() {
        if ch == needle {
            if let Some(next) = query_iter.next() {
                needle = next;
            } else {
                return Some(1_000);
            }
        }
    }

    None
}

/// Rank items by how well their keys match the query.
///
/// Returns a vec of indices sorted by descending match score, with ties broken
/// by ascending original index to maintain stable order.
fn rank_snapshot_matches<T>(items: &[T], query: &str, key: impl Fn(&T) -> &str) -> Vec<usize> {
    let mut ranked = items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| score_snapshot_match(key(item), query).map(|score| (idx, score)))
        .collect::<Vec<_>>();

    ranked.sort_unstable_by(|(lhs_idx, lhs_score), (rhs_idx, rhs_score)| {
        rhs_score.cmp(lhs_score).then_with(|| lhs_idx.cmp(rhs_idx))
    });

    ranked.into_iter().map(|(idx, _)| idx).collect()
}

/// Convert a usize to i64, saturating at i64::MAX to avoid overflow.
fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Run a fuzzy find across indexed files and/or directories.
///
/// For imported snapshots this uses lightweight in-memory ranking over
/// restored path metadata.
pub fn stream_find(
    query: &str,
    files_only: bool,
    dirs_only: bool,
    verbose: bool,
    limit: usize,
    span: Span,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let snapshot = require_runtime(span)?;

    if let (Some(restored_files), Some(restored_dirs)) = (
        snapshot.restored_files.clone(),
        snapshot.restored_dirs.clone(),
    ) {
        let mut ranked_files = restored_files
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                score_snapshot_match(&item.relative_path, query).map(|score| ("file", idx, score))
            })
            .collect::<Vec<_>>();

        let mut ranked_dirs = restored_dirs
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                score_snapshot_match(&item.relative_path, query).map(|score| ("dir", idx, score))
            })
            .collect::<Vec<_>>();

        if files_only {
            ranked_dirs.clear();
        }
        if dirs_only {
            ranked_files.clear();
        }

        let mut combined = Vec::with_capacity(ranked_files.len().saturating_add(ranked_dirs.len()));
        combined.extend(ranked_files);
        combined.extend(ranked_dirs);
        combined.sort_unstable_by(
            |(lhs_kind, lhs_idx, lhs_score), (rhs_kind, rhs_idx, rhs_score)| {
                rhs_score
                    .cmp(lhs_score)
                    .then_with(|| lhs_kind.cmp(rhs_kind))
                    .then_with(|| lhs_idx.cmp(rhs_idx))
            },
        );
        combined.truncate(limit);

        let find_data = combined
            .into_iter()
            .enumerate()
            .map(|(rank, (kind, idx, score))| {
                let path = if kind == "file" {
                    restored_files
                        .get(idx)
                        .map(|row| row.relative_path.clone())
                        .unwrap_or_default()
                } else {
                    restored_dirs
                        .get(idx)
                        .map(|row| row.relative_path.clone())
                        .unwrap_or_default()
                };

                let mut cols = vec![
                    ("kind".to_string(), Value::string(kind, span)),
                    ("path".to_string(), Value::string(path, span)),
                    ("rank".to_string(), Value::int(usize_to_i64(rank + 1), span)),
                    ("score".to_string(), Value::int(score, span)),
                ];

                if verbose {
                    if !files_only && !dirs_only {
                        cols.push((
                            "score_details".to_string(),
                            Value::record(
                                [
                                    ("base_score".to_string(), Value::int(score, span)),
                                    ("filename_bonus".to_string(), Value::int(0, span)),
                                    ("special_filename_bonus".to_string(), Value::int(0, span)),
                                    ("frecency_boost".to_string(), Value::int(0, span)),
                                ]
                                .into_iter()
                                .collect(),
                                span,
                            ),
                        ));
                    } else if files_only {
                        cols.push(("match_type".to_string(), Value::string("snapshot", span)));
                    } else {
                        cols.push(("exact_match".to_string(), Value::bool(false, span)));
                    }
                }

                Value::record(cols.into_iter().collect(), span)
            })
            .collect::<Vec<_>>();

        let stream_signals = signals.clone();
        let stream = find_data.into_iter().map(move |value| {
            if let Err(err) = stream_signals.check(&span) {
                return Value::error(err, span);
            }
            value
        });

        return Ok(PipelineData::ListStream(
            ListStream::new(stream, span, signals.clone()),
            Some(PipelineMetadata::default()),
        ));
    }

    let guard = snapshot
        .shared_picker
        .read()
        .map_err(|err| fff_error(err, span))?;
    let picker = guard.as_ref().ok_or_else(|| {
        ShellError::Generic(GenericError::new(
            "idx is not initialized",
            "run `idx init <path>` first",
            span,
        ))
    })?;

    let parser = QueryParser::default();
    let parsed = parser.parse(query);
    let options = FuzzySearchOptions {
        max_threads: 0,
        current_file: None,
        project_path: None,
        combo_boost_score_multiplier: 0,
        min_combo_count: 0,
        pagination: PaginationArgs { offset: 0, limit },
    };

    let find_data: Vec<Value> = if !files_only && !dirs_only {
        let result = picker.fuzzy_search_mixed(&parsed, None, options);

        result
            .items
            .iter()
            .zip(result.scores.iter())
            .enumerate()
            .map(|(rank, (item, score))| {
                let (kind, path) = match item {
                    MixedItemRef::File(file) => ("file", file.relative_path(picker)),
                    MixedItemRef::Dir(dir) => ("dir", dir.relative_path(picker)),
                };

                let mut cols = vec![
                    ("kind".to_string(), Value::string(kind, span)),
                    ("path".to_string(), Value::string(path, span)),
                    ("rank".to_string(), Value::int(usize_to_i64(rank + 1), span)),
                    (
                        "score".to_string(),
                        Value::int(i64::from(score.total), span),
                    ),
                ];

                if verbose {
                    cols.push((
                        "score_details".to_string(),
                        Value::record(
                            [
                                (
                                    "base_score".to_string(),
                                    Value::int(i64::from(score.base_score), span),
                                ),
                                (
                                    "filename_bonus".to_string(),
                                    Value::int(i64::from(score.filename_bonus), span),
                                ),
                                (
                                    "special_filename_bonus".to_string(),
                                    Value::int(i64::from(score.special_filename_bonus), span),
                                ),
                                (
                                    "frecency_boost".to_string(),
                                    Value::int(i64::from(score.frecency_boost), span),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                            span,
                        ),
                    ));
                }

                Value::record(cols.into_iter().collect(), span)
            })
            .collect()
    } else if dirs_only {
        let result = picker.fuzzy_search_directories(&parsed, options);
        result
            .items
            .iter()
            .zip(result.scores.iter())
            .enumerate()
            .map(|(rank, (item, score))| {
                let mut cols = vec![
                    ("kind".to_string(), Value::string("dir", span)),
                    (
                        "path".to_string(),
                        Value::string(item.relative_path(picker), span),
                    ),
                    ("rank".to_string(), Value::int(usize_to_i64(rank + 1), span)),
                    (
                        "score".to_string(),
                        Value::int(i64::from(score.total), span),
                    ),
                ];

                if verbose {
                    cols.push((
                        "exact_match".to_string(),
                        Value::bool(score.exact_match, span),
                    ));
                }

                Value::record(cols.into_iter().collect(), span)
            })
            .collect()
    } else {
        let result = picker.fuzzy_search(&parsed, None, options);
        result
            .items
            .iter()
            .zip(result.scores.iter())
            .enumerate()
            .map(|(rank, (item, score))| {
                let mut cols = vec![
                    ("kind".to_string(), Value::string("file", span)),
                    (
                        "path".to_string(),
                        Value::string(item.relative_path(picker), span),
                    ),
                    ("rank".to_string(), Value::int(usize_to_i64(rank + 1), span)),
                    (
                        "score".to_string(),
                        Value::int(i64::from(score.total), span),
                    ),
                ];

                if verbose {
                    cols.push((
                        "match_type".to_string(),
                        Value::string(score.match_type, span),
                    ));
                }

                Value::record(cols.into_iter().collect(), span)
            })
            .collect()
    };

    drop(guard);

    let stream_signals = signals.clone();
    let stream = find_data.into_iter().map(move |value| {
        if let Err(err) = stream_signals.check(&span) {
            return Value::error(err, span);
        }
        value
    });

    Ok(PipelineData::ListStream(
        ListStream::new(stream, span, signals.clone()),
        Some(PipelineMetadata::default()),
    ))
}

#[cfg(feature = "sqlite")]
#[derive(Debug, Serialize, Deserialize)]
pub struct IdxSnapshotFile {
    pub relative_path: String,
    pub full_path: String,
    pub file_name: String,
    pub directory: String,
    pub size: u64,
    pub modified: u64,
    pub access_frecency_score: i16,
    pub modification_frecency_score: i16,
    pub is_binary: bool,
    pub is_deleted: bool,
    pub is_overflow: bool,
}

#[cfg(feature = "sqlite")]
#[derive(Debug)]
pub struct IdxSnapshotDir {
    pub relative_path: String,
    pub full_path: String,
    pub last_segment_offset: u16,
    pub max_access_frecency: i32,
    pub is_overflow: bool,
}

/// Persist the current idx runtime into a SQLite snapshot file.
#[cfg(feature = "sqlite")]
pub fn store_snapshot(path: &Path, span: Span) -> Result<Value, ShellError> {
    let snapshot = require_runtime(span)?;
    let guard = read_picker_guard(&snapshot.shared_picker, span)?;
    let picker = picker_from_guard(&guard, span)?;

    let files = picker
        .get_files()
        .iter()
        .map(|item| IdxSnapshotFile {
            relative_path: item.relative_path(picker),
            full_path: item
                .absolute_path(picker, &snapshot.base_path)
                .to_string_lossy()
                .to_string(),
            file_name: item.file_name(picker),
            directory: item.dir_str(picker),
            size: item.size,
            modified: item.modified,
            access_frecency_score: item.access_frecency_score,
            modification_frecency_score: item.modification_frecency_score,
            is_binary: item.is_binary(),
            is_deleted: item.is_deleted(),
            is_overflow: item.is_overflow(),
        })
        .collect::<Vec<_>>();

    let dirs = picker
        .get_dirs()
        .iter()
        .map(|item| IdxSnapshotDir {
            relative_path: item.relative_path(picker),
            full_path: item
                .absolute_path(picker, &snapshot.base_path)
                .to_string_lossy()
                .to_string(),
            last_segment_offset: item.last_segment_offset(),
            max_access_frecency: item.max_access_frecency(),
            is_overflow: item.is_overflow(),
        })
        .collect::<Vec<_>>();

    // Create or open SQLite database
    let conn = Connection::open(path).map_err(|err| {
        ShellError::Generic(GenericError::new(
            "idx snapshot database open failed",
            err.to_string(),
            span,
        ))
    })?;

    // Create schema
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS metadata (
            id INTEGER PRIMARY KEY,
            version INTEGER NOT NULL,
            base_path TEXT NOT NULL,
            watch BOOLEAN NOT NULL,
            file_count INTEGER NOT NULL,
            dir_count INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY,
            relative_path TEXT NOT NULL,
            full_path TEXT NOT NULL,
            file_name TEXT NOT NULL,
            directory TEXT NOT NULL,
            size INTEGER NOT NULL,
            modified INTEGER NOT NULL,
            access_frecency_score INTEGER NOT NULL,
            modification_frecency_score INTEGER NOT NULL,
            is_binary BOOLEAN NOT NULL,
            is_deleted BOOLEAN NOT NULL,
            is_overflow BOOLEAN NOT NULL
        );
        CREATE TABLE IF NOT EXISTS dirs (
            id INTEGER PRIMARY KEY,
            relative_path TEXT NOT NULL,
            full_path TEXT NOT NULL,
            last_segment_offset INTEGER NOT NULL,
            max_access_frecency INTEGER NOT NULL,
            is_overflow BOOLEAN NOT NULL
        );",
    )
    .map_err(|err| {
        ShellError::Generic(GenericError::new(
            "idx snapshot schema creation failed",
            err.to_string(),
            span,
        ))
    })?;

    // Clear existing data
    let tx = conn.unchecked_transaction().map_err(|err| {
        ShellError::Generic(GenericError::new(
            "idx snapshot transaction failed",
            err.to_string(),
            span,
        ))
    })?;
    tx.execute("DELETE FROM metadata", []).map_err(|err| {
        ShellError::Generic(GenericError::new(
            "idx snapshot clear failed",
            err.to_string(),
            span,
        ))
    })?;
    tx.execute("DELETE FROM files", []).map_err(|err| {
        ShellError::Generic(GenericError::new(
            "idx snapshot clear failed",
            err.to_string(),
            span,
        ))
    })?;
    tx.execute("DELETE FROM dirs", []).map_err(|err| {
        ShellError::Generic(GenericError::new(
            "idx snapshot clear failed",
            err.to_string(),
            span,
        ))
    })?;

    // Insert metadata
    tx.execute(
        "INSERT INTO metadata (version, base_path, watch, file_count, dir_count) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![1, snapshot.base_path.display().to_string(), snapshot.watch, files.len(), dirs.len()],
    )
    .map_err(|err| {
        ShellError::Generic(GenericError::new(
            "idx snapshot metadata insert failed",
            err.to_string(),
            span,
        ))
    })?;

    // Insert files
    for file in &files {
        tx.execute(
            "INSERT INTO files (relative_path, full_path, file_name, directory, size, modified, access_frecency_score, modification_frecency_score, is_binary, is_deleted, is_overflow) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                file.relative_path,
                file.full_path,
                file.file_name,
                file.directory,
                file.size,
                file.modified,
                file.access_frecency_score,
                file.modification_frecency_score,
                file.is_binary,
                file.is_deleted,
                file.is_overflow,
            ],
        )
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "idx snapshot file insert failed",
                err.to_string(),
                span,
            ))
        })?;
    }

    // Insert dirs
    for dir in &dirs {
        tx.execute(
            "INSERT INTO dirs (relative_path, full_path, last_segment_offset, max_access_frecency, is_overflow) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                dir.relative_path,
                dir.full_path,
                dir.last_segment_offset,
                dir.max_access_frecency,
                dir.is_overflow,
            ],
        )
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "idx snapshot dir insert failed",
                err.to_string(),
                span,
            ))
        })?;
    }

    tx.commit().map_err(|err| {
        ShellError::Generic(GenericError::new(
            "idx snapshot commit failed",
            err.to_string(),
            span,
        ))
    })?;

    Ok(Value::record(
        [
            ("stored".to_string(), Value::bool(true, span)),
            (
                "path".to_string(),
                Value::string(path.to_string_lossy().to_string(), span),
            ),
            (
                "file_count".to_string(),
                Value::int(i64::try_from(files.len()).unwrap_or(i64::MAX), span),
            ),
            (
                "dir_count".to_string(),
                Value::int(i64::try_from(dirs.len()).unwrap_or(i64::MAX), span),
            ),
            (
                "base_path".to_string(),
                Value::string(snapshot.base_path.display().to_string(), span),
            ),
            ("format".to_string(), Value::string("sqlite", span)),
        ]
        .into_iter()
        .collect(),
        span,
    ))
}

/// Restore idx runtime from a SQLite snapshot file.
///
/// The restored runtime is immediately queryable by `idx files`, `idx dirs`,
/// `idx find`, and `idx status` without a filesystem scan.
#[cfg(feature = "sqlite")]
pub fn restore_snapshot(path: &Path, no_watch: bool, span: Span) -> Result<Value, ShellError> {
    // Open SQLite database
    let conn = Connection::open(path).map_err(|err| {
        ShellError::Generic(GenericError::new(
            "idx snapshot database open failed",
            err.to_string(),
            span,
        ))
    })?;

    // Read metadata
    let metadata = conn
        .query_row(
            "SELECT version, base_path, watch, file_count, dir_count FROM metadata LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, usize>(3)?,
                    row.get::<_, usize>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "idx snapshot metadata query failed",
                err.to_string(),
                span,
            ))
        })?
        .ok_or_else(|| {
            ShellError::Generic(GenericError::new(
                "idx snapshot is empty",
                "the snapshot database contains no metadata",
                span,
            ))
        })?;

    let (version, base_path_str, _watch, _file_count, _dir_count) = metadata;

    if version != 1 {
        return Err(ShellError::Generic(GenericError::new(
            "unsupported idx snapshot version",
            format!("expected version 1, found {}", version),
            span,
        )));
    }

    // Read all files from snapshot (offline restoration)
    let mut stmt = conn
        .prepare("SELECT relative_path, full_path, file_name, directory, size, modified, access_frecency_score, modification_frecency_score, is_binary, is_deleted, is_overflow FROM files")
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "idx snapshot file query failed",
                err.to_string(),
                span,
            ))
        })?;

    let files: Vec<IdxSnapshotFile> = stmt
        .query_map([], |row| {
            Ok(IdxSnapshotFile {
                relative_path: row.get(0)?,
                full_path: row.get(1)?,
                file_name: row.get(2)?,
                directory: row.get(3)?,
                size: row.get(4)?,
                modified: row.get(5)?,
                access_frecency_score: row.get(6)?,
                modification_frecency_score: row.get(7)?,
                is_binary: row.get(8)?,
                is_deleted: row.get(9)?,
                is_overflow: row.get(10)?,
            })
        })
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "idx snapshot file query failed",
                err.to_string(),
                span,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "idx snapshot file collection failed",
                err.to_string(),
                span,
            ))
        })?;

    // Read all dirs from snapshot (offline restoration)
    let mut stmt = conn
        .prepare("SELECT relative_path, full_path, last_segment_offset, max_access_frecency, is_overflow FROM dirs")
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "idx snapshot dir query failed",
                err.to_string(),
                span,
            ))
        })?;

    let dirs: Vec<IdxSnapshotDir> = stmt
        .query_map([], |row| {
            Ok(IdxSnapshotDir {
                relative_path: row.get(0)?,
                full_path: row.get(1)?,
                last_segment_offset: row.get(2)?,
                max_access_frecency: row.get(3)?,
                is_overflow: row.get(4)?,
            })
        })
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "idx snapshot dir query failed",
                err.to_string(),
                span,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "idx snapshot dir collection failed",
                err.to_string(),
                span,
            ))
        })?;

    // Compute arena memory from restored data
    let mut arena_bytes_base = 0usize;
    let mut arena_bytes_overflow = 0usize;

    for file in &files {
        let size = file.relative_path.len()
            + file.full_path.len()
            + file.file_name.len()
            + file.directory.len();
        if file.is_overflow {
            arena_bytes_overflow = arena_bytes_overflow.saturating_add(size);
        } else {
            arena_bytes_base = arena_bytes_base.saturating_add(size);
        }
    }

    for dir in &dirs {
        let size = dir.relative_path.len() + dir.full_path.len();
        if dir.is_overflow {
            arena_bytes_overflow = arena_bytes_overflow.saturating_add(size);
        } else {
            arena_bytes_base = arena_bytes_base.saturating_add(size);
        }
    }

    let restored_files = Arc::new(
        files
            .iter()
            .map(|row| IdxRestoredFile {
                relative_path: row.relative_path.clone(),
                full_path: row.full_path.clone(),
                file_name: row.file_name.clone(),
                directory: row.directory.clone(),
                size: row.size,
                modified: row.modified,
            })
            .collect::<Vec<_>>(),
    );

    let restored_dirs = Arc::new(
        dirs.iter()
            .map(|row| IdxRestoredDir {
                relative_path: row.relative_path.clone(),
                full_path: row.full_path.clone(),
            })
            .collect::<Vec<_>>(),
    );

    let mut guard = runtime().lock().map_err(|_| {
        ShellError::Generic(GenericError::new(
            "idx runtime lock failed",
            "idx runtime lock poisoned",
            span,
        ))
    })?;

    let shared_picker = SharedFilePicker::default();
    let shared_frecency = SharedFrecency::noop();

    // Try to initialize a live picker for the base_path to enable grep search.
    // This happens in the background; if the path no longer exists, grep will
    // simply return no results.
    let _ = FilePicker::new_with_shared_state(
        shared_picker.clone(),
        shared_frecency,
        FilePickerOptions {
            base_path: base_path_str.clone(),
            cache_budget: None,
            enable_content_indexing: false,
            enable_fs_root_scanning: false,
            enable_home_dir_scanning: true,
            enable_mmap_cache: false,
            follow_symlinks: false,
            mode: FFFMode::Ai,
            watch: !no_watch,
        },
    );

    // Wait for the picker scan to complete so grep can search indexed content.
    let _ = shared_picker.wait_for_scan(Duration::from_secs(300));

    let previous = guard.replace(IdxRuntime {
        base_path: PathBuf::from(&base_path_str),
        watch: false,
        shared_picker: shared_picker.clone(),
        scan_start_time: Instant::now(),
        scan_completed: Arc::new(AtomicBool::new(true)),
        scan_duration_ns: Arc::new(AtomicU64::new(0)),
        restored_files: Some(restored_files.clone()),
        restored_dirs: Some(restored_dirs.clone()),
        restored_arena_bytes_base: arena_bytes_base,
        restored_arena_bytes_overflow: arena_bytes_overflow,
    });
    drop(guard);

    if let Some(old_runtime) = previous {
        let _ = shutdown_shared_picker(&old_runtime.shared_picker, span);
    }

    let status = current_status(None);

    Ok(Value::record(
        [
            ("restored".to_string(), Value::bool(true, span)),
            (
                "source_path".to_string(),
                Value::string(path.to_string_lossy().to_string(), span),
            ),
            (
                "base_path".to_string(),
                Value::string(base_path_str.clone(), span),
            ),
            ("watch".to_string(), Value::bool(false, span)),
            (
                "rehydration_mode".to_string(),
                Value::string("snapshot_runtime_restored", span),
            ),
            ("status".to_string(), status.to_value(span)),
            (
                "restored_files".to_string(),
                Value::int(
                    i64::try_from(restored_files.len()).unwrap_or(i64::MAX),
                    span,
                ),
            ),
            (
                "restored_dirs".to_string(),
                Value::int(i64::try_from(restored_dirs.len()).unwrap_or(i64::MAX), span),
            ),
            ("format".to_string(), Value::string("sqlite", span)),
        ]
        .into_iter()
        .collect(),
        span,
    ))
}

/// Search indexed file contents (`idx search`).
///
/// When backed by an imported snapshot, grep searches the live filesystem for the base_path.
/// If the base_path no longer exists or files have been moved, grep will return no results.
pub fn stream_grep(
    patterns: &[String],
    mode: GrepMode,
    page_limit: usize,
    span: Span,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let snapshot = require_runtime(span)?;

    let guard = read_picker_guard(&snapshot.shared_picker, span)?;
    let picker = picker_from_guard(&guard, span)?;

    let options = GrepSearchOptions {
        mode,
        page_limit,
        ..Default::default()
    };

    let result = if patterns.len() == 1 {
        let parser = QueryParser::default();
        let query = parser.parse(&patterns[0]);
        picker.grep(&query, &options)
    } else {
        let refs = patterns.iter().map(String::as_str).collect::<Vec<_>>();
        picker.multi_grep(&refs, &[], &options)
    };

    let file_paths = result
        .files
        .iter()
        .map(|f| f.relative_path(picker))
        .collect::<Vec<_>>();
    let matches = result.matches;

    drop(guard);

    let stream_signals = signals.clone();
    let stream = matches.into_iter().map(move |item| {
        if let Err(err) = stream_signals.check(&span) {
            return Value::error(err, span);
        }

        let path = file_paths
            .get(item.file_index)
            .cloned()
            .unwrap_or_else(|| "<unknown>".to_string());

        let offsets = item
            .match_byte_offsets
            .iter()
            .map(|(start, end)| {
                let start = i64::from(*start);
                let end = i64::from(*end);
                Value::record(
                    [
                        (
                            "start".to_string(),
                            Value::int(start + item.byte_offset as i64, span),
                        ),
                        (
                            "end".to_string(),
                            Value::int(item.byte_offset as i64 + end, span),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    span,
                )
            })
            .collect::<Vec<_>>();

        Value::record(
            [
                ("path".to_string(), Value::string(path, span)),
                (
                    "line_number".to_string(),
                    Value::int(i64::try_from(item.line_number).unwrap_or(i64::MAX), span),
                ),
                (
                    "column".to_string(),
                    Value::int(usize_to_i64(item.col), span),
                ),
                (
                    "byte_offset".to_string(),
                    Value::int(i64::try_from(item.byte_offset).unwrap_or(i64::MAX), span),
                ),
                (
                    "line".to_string(),
                    Value::string(item.line_content.clone(), span),
                ),
                ("matches".to_string(), Value::list(offsets, span)),
            ]
            .into_iter()
            .collect(),
            span,
        )
    });

    Ok(PipelineData::ListStream(
        ListStream::new(stream, span, signals.clone()),
        Some(PipelineMetadata::default()),
    ))
}
