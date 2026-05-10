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

pub struct IdxRuntime {
    pub base_path: PathBuf,
    pub watch: bool,
    pub shared_picker: SharedFilePicker,
    pub scan_start_time: Instant,
    pub scan_completed: Arc<AtomicBool>,
    pub scan_duration_ms: Arc<AtomicU64>,
}

/// A cloned snapshot of the current runtime state, obtained while holding the
/// global mutex for the minimum amount of time. All fields are cheaply
/// cloneable (Arc / Copy), so callers can work with this after the lock is
/// released.
pub type RuntimeSnapshot = IdxRuntime;

#[derive(Clone, Debug, Default)]
pub struct IdxStatus {
    pub initialized: bool,
    pub base_path: String,
    pub watch: bool,
    pub scanning: bool,
    pub scan_duration_ms: u128,
    pub files: usize,
    pub dirs: usize,
    pub arena_bytes_base: usize,
    pub arena_bytes_overflow: usize,
    pub arena_bytes_untracked: usize,
}

impl IdxStatus {
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
                    "scan_duration_ms".to_string(),
                    Value::int(
                        i64::try_from(self.scan_duration_ms).unwrap_or(i64::MAX),
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
                    "arena_bytes_base".to_string(),
                    Value::filesize(
                        i64::try_from(self.arena_bytes_base).unwrap_or(i64::MAX),
                        span,
                    ),
                ),
                (
                    "arena_bytes_overflow".to_string(),
                    Value::filesize(
                        i64::try_from(self.arena_bytes_overflow).unwrap_or(i64::MAX),
                        span,
                    ),
                ),
                (
                    "arena_bytes_untracked".to_string(),
                    Value::filesize(
                        i64::try_from(self.arena_bytes_untracked).unwrap_or(i64::MAX),
                        span,
                    ),
                ),
                (
                    "arena_bytes_total".to_string(),
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

fn fff_error<E: std::fmt::Display>(err: E, span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(
        "idx operation failed",
        err.to_string(),
        span,
    ))
}

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

fn idx_status_from_picker(
    base_path: &Path,
    watch: bool,
    picker: &FilePicker,
    scan_duration_ms: u128,
) -> IdxStatus {
    IdxStatus {
        initialized: true,
        base_path: base_path.display().to_string(),
        watch,
        scanning: picker.is_scan_active(),
        scan_duration_ms,
        files: picker.get_files().len(),
        dirs: picker.get_dirs().len(),
        arena_bytes_base: picker.arena_bytes().0,
        arena_bytes_overflow: picker.arena_bytes().1,
        arena_bytes_untracked: picker.arena_bytes().2,
    }
}

fn elapsed_ms(scan_start: Instant) -> u64 {
    u64::try_from(scan_start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn freeze_scan_duration_if_needed(
    scan_completed: &AtomicBool,
    scan_duration_ms: &AtomicU64,
    scan_start: Instant,
    scanning: bool,
) -> u128 {
    if scan_completed.load(Ordering::Acquire) {
        return u128::from(scan_duration_ms.load(Ordering::Acquire));
    }

    let elapsed = elapsed_ms(scan_start);
    if !scanning {
        if scan_completed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            scan_duration_ms.store(elapsed, Ordering::Release);
            return u128::from(elapsed);
        }

        return u128::from(scan_duration_ms.load(Ordering::Acquire));
    }

    u128::from(elapsed)
}

fn runtime_snapshot() -> Option<RuntimeSnapshot> {
    let guard = runtime().lock().ok()?;
    let runtime = guard.as_ref()?;
    Some(RuntimeSnapshot {
        base_path: runtime.base_path.clone(),
        watch: runtime.watch,
        shared_picker: runtime.shared_picker.clone(),
        scan_start_time: runtime.scan_start_time,
        scan_completed: runtime.scan_completed.clone(),
        scan_duration_ms: runtime.scan_duration_ms.clone(),
    })
}

fn require_runtime(span: Span) -> Result<RuntimeSnapshot, ShellError> {
    runtime_snapshot().ok_or_else(|| {
        ShellError::Generic(GenericError::new(
            "idx is not initialized",
            "run `idx init <path>` first",
            span,
        ))
    })
}

pub fn current_status(scan_start_override: Option<Instant>) -> IdxStatus {
    let Some(snapshot) = runtime_snapshot() else {
        return IdxStatus::default();
    };

    let scan_start = scan_start_override.unwrap_or(snapshot.scan_start_time);

    let Ok(guard) = snapshot.shared_picker.read() else {
        let duration = freeze_scan_duration_if_needed(
            &snapshot.scan_completed,
            &snapshot.scan_duration_ms,
            scan_start,
            false,
        );
        return IdxStatus {
            initialized: true,
            base_path: snapshot.base_path.display().to_string(),
            watch: snapshot.watch,
            scanning: false,
            scan_duration_ms: duration,
            ..Default::default()
        };
    };

    guard
        .as_ref()
        .map(|picker| {
            let scanning = picker.is_scan_active();
            let duration = freeze_scan_duration_if_needed(
                &snapshot.scan_completed,
                &snapshot.scan_duration_ms,
                scan_start,
                scanning,
            );
            idx_status_from_picker(&snapshot.base_path, snapshot.watch, picker, duration)
        })
        .unwrap_or_default()
}

pub fn init_runtime(
    path: &Path,
    watch: bool,
    wait: bool,
    span: Span,
) -> Result<IdxStatus, ShellError> {
    let shared_picker = SharedFilePicker::default();
    let shared_frecency = SharedFrecency::noop();

    FilePicker::new_with_shared_state(
        shared_picker.clone(),
        shared_frecency,
        FilePickerOptions {
            base_path: path.display().to_string(),
            enable_mmap_cache: false,
            enable_content_indexing: false,
            mode: FFFMode::Ai,
            cache_budget: None,
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
        scan_duration_ms: Arc::new(AtomicU64::new(0)),
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
        if !shared_picker.wait_for_scan(WAIT_TIMEOUT) {
            return Err(ShellError::Generic(GenericError::new(
                "idx scan timed out",
                "timed out waiting for the initial scan to finish (300 s). The index is still available with partial results.",
                span,
            )));
        }
        if watch && !shared_picker.wait_for_watcher(WAIT_TIMEOUT) {
            return Err(ShellError::Generic(GenericError::new(
                "idx watcher startup timed out",
                "timed out waiting for the background filesystem watcher to become ready (300 s).",
                span,
            )));
        }
    }
    let status = IdxStatus {
        initialized: true,
        base_path: path.display().to_string(),
        watch,
        scanning: true,
        ..Default::default()
    };

    Ok(status)
}

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

pub fn stream_dirs(span: Span, signals: &Signals) -> Result<PipelineData, ShellError> {
    let snapshot = require_runtime(span)?;
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

    let base_path = snapshot.base_path.clone();
    let dirs_data: Vec<_> = picker
        .get_dirs()
        .iter()
        .map(|item| {
            let rel_path = item.relative_path(picker);
            let full_path = item.absolute_path(picker, &base_path);

            Value::record(
                [
                    ("relative_path".to_string(), Value::string(rel_path, span)),
                    (
                        "full_path".to_string(),
                        Value::string(full_path.to_string_lossy().to_string(), span),
                    ),
                ]
                .into_iter()
                .collect(),
                span,
            )
        })
        .collect();

    drop(guard);

    let stream_signals = signals.clone();
    let stream = dirs_data.into_iter().map(move |value| {
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

pub fn stream_files(
    path: Option<String>,
    span: Span,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let snapshot = require_runtime(span)?;
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

    let base_path = snapshot.base_path.clone();

    let files_data: Vec<_> = if let Some(path_str) = path {
        let lookup_path = Path::new(&path_str);

        // Try relative path first, then strip base_path prefix from absolute paths.
        let item = picker.get_file_by_path(lookup_path).or_else(|| {
            if lookup_path.is_absolute() {
                lookup_path
                    .strip_prefix(&base_path)
                    .ok()
                    .and_then(|rel| picker.get_file_by_path(rel))
            } else {
                None
            }
        });

        item.map(|i| vec![build_file_record(i, picker, &base_path, span)])
            .unwrap_or_default()
    } else {
        picker
            .get_files()
            .iter()
            .map(|item| build_file_record(item, picker, &base_path, span))
            .collect()
    };

    drop(guard);

    let stream_signals = signals.clone();
    let stream = files_data.into_iter().map(move |value| {
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

fn build_file_record(
    item: &fff_search::FileItem,
    picker: &FilePicker,
    base_path: &Path,
    span: Span,
) -> Value {
    let full_path = item.absolute_path(picker, base_path);
    Value::record(
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
                Value::string(item.file_name(picker), span),
            ),
            (
                "directory".to_string(),
                Value::string(item.dir_str(picker), span),
            ),
            (
                "size".to_string(),
                Value::int(i64::try_from(item.size).unwrap_or(i64::MAX), span),
            ),
            (
                "modified".to_string(),
                Value::int(i64::try_from(item.modified).unwrap_or(i64::MAX), span),
            ),
        ]
        .into_iter()
        .collect(),
        span,
    )
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

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

#[cfg(feature = "sqlite")]
pub fn store_snapshot(path: &Path, span: Span) -> Result<Value, ShellError> {
    let snapshot = require_runtime(span)?;
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

#[cfg(feature = "sqlite")]
pub fn restore_snapshot(path: &Path, span: Span) -> Result<Value, ShellError> {
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

    let (version, base_path_str, watch, file_count, dir_count) = metadata;

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

    // Create status from restored snapshot data (no live scanning)
    let status = IdxStatus {
        initialized: true,
        base_path: base_path_str.clone(),
        watch,
        scanning: false, // offline restore doesn't scan
        scan_duration_ms: 0,
        files: file_count,
        dirs: dir_count,
        arena_bytes_base,
        arena_bytes_overflow,
        arena_bytes_untracked: 0,
    };

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
            ("watch".to_string(), Value::bool(watch, span)),
            (
                "rehydration_mode".to_string(),
                Value::string("offline_from_snapshot_rows", span),
            ),
            ("status".to_string(), status.to_value(span)),
            (
                "restored_files".to_string(),
                Value::int(i64::try_from(files.len()).unwrap_or(i64::MAX), span),
            ),
            (
                "restored_dirs".to_string(),
                Value::int(i64::try_from(dirs.len()).unwrap_or(i64::MAX), span),
            ),
            ("format".to_string(), Value::string("sqlite", span)),
        ]
        .into_iter()
        .collect(),
        span,
    ))
}

pub fn stream_grep(
    patterns: &[String],
    mode: GrepMode,
    page_limit: usize,
    span: Span,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let snapshot = require_runtime(span)?;
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

    let grep_data: Vec<Value> = result
        .matches
        .iter()
        .map(|item| {
            let path = result
                .files
                .get(item.file_index)
                .map(|f| f.relative_path(picker))
                .unwrap_or_else(|| "<unknown>".to_string());

            let offsets = item
                .match_byte_offsets
                .iter()
                .map(|(start, end)| {
                    let start = i64::from(*start);
                    let end = i64::from(*end);
                    Value::record(
                        [
                            // ("start".to_string(), Value::int(start, span)),
                            // ("end".to_string(), Value::int(end, span)),

                            // I'm guessing calculated offset to match start, end is more valuable
                            // than just the offset to start and end of the match
                            (
                                "start".to_string(),
                                // byte_offset is the offset to the beginning of the line,
                                // so match start is offset + start
                                Value::int(start + item.byte_offset as i64, span),
                            ),
                            (
                                "end".to_string(),
                                // byte_offset is the offset to the beginning of the line,
                                // so match end is offset + start + length or offset + end
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
        })
        .collect();

    drop(guard);

    let stream_signals = signals.clone();
    let stream = grep_data.into_iter().map(move |value| {
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
