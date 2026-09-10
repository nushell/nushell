//! Called from `build.rs` and the tests.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Filename shadow-rs writes into `OUT_DIR`.
pub(crate) const SHADOW_RS_OUTPUT: &str = "shadow.rs";

/// Restore previous bytes when only `BUILD_TIME` changed, and rewind mtime
/// so cargo does not rebuild this crate.
pub(crate) fn stabilize_shadow_rs_output(shadow_path: &Path, previous: Option<&[u8]>) {
    let Ok(regenerated) = std::fs::read(shadow_path) else {
        return;
    };

    let contents: &[u8] = match previous {
        Some(prev) if strip_build_time(prev) == strip_build_time(&regenerated) => prev,
        Some(_) => return,
        None => &regenerated,
    };

    let Some(mtime) = mtime_before_script_start() else {
        return;
    };

    if let Err(err) = write_with_mtime(shadow_path, contents, mtime) {
        println!("cargo:warning=failed to stabilize {SHADOW_RS_OUTPUT} mtime: {err}");
    }
}

/// Older than cargo's build-script start stamp. `now()` in `main` is too late.
pub(crate) fn mtime_before_script_start() -> Option<SystemTime> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.metadata().ok())
        .and_then(|meta| meta.modified().ok())
        .or_else(|| SystemTime::now().checked_sub(Duration::from_secs(2)))
}

fn write_with_mtime(path: &Path, contents: &[u8], mtime: SystemTime) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(contents)?;
    file.flush()?;
    file.set_modified(mtime)
}

/// Drop timestamp lines so two generated files can be compared.
pub(crate) fn strip_build_time(contents: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(contents)
        .lines()
        .filter(|line| !line.contains("BUILD_TIME") && !line.starts_with("// Generation time:"))
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes()
}
