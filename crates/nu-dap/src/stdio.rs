//! Process stdio plumbing.
//!
//! This process's stdin/stdout carry the DAP protocol, but an embedded nu
//! evaluation writes to process stdout from several places that no stack
//! redirection reaches (statement drains print byte streams directly,
//! externals inherit process handles, stray eprintln!s...). And an external
//! that *reads* inherited stdin would steal DAP frames or block forever.
//!
//! Fix, applied once at startup:
//! - stdin: duplicate the real handle for DAP reading, then point the
//!   process-level stdin at the null device — children see immediate EOF.
//! - stdout/stderr: duplicate the real stdout for DAP writing, then point
//!   the process-level stdout/stderr at capture pipes whose forwarder
//!   threads re-emit everything as DAP `output` events.
//!
//! Because this process keeps the pipe write ends alive forever (they ARE
//! its std handles), forwarders can't rely on EOF. `flush_output` writes a
//! marker through the pipes and waits until the forwarders have seen it —
//! call it before emitting `terminated` so late output isn't lost.

use std::fs::File;
use std::io::PipeReader;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

use crate::dap::protocol::DapWriter;

const MARKER: &[u8] = b"\x01<NU-DAP-FLUSH>\x01";

/// Set once the forwarders own process stdout/stderr. Until then the marker
/// bytes would land on the host's real stdio, so `flush_output` is a no-op.
static CAPTURING: AtomicBool = AtomicBool::new(false);

static FLUSH_SEEN: OnceLock<(Mutex<u64>, Condvar)> = OnceLock::new();

fn flush_state() -> &'static (Mutex<u64>, Condvar) {
    FLUSH_SEEN.get_or_init(|| (Mutex::new(0), Condvar::new()))
}

/// Rolling tails of captured process output (stdout, stderr) — inspectable
/// in the debugger's Process scope and attached to external-command errors.
static RECENT: OnceLock<Mutex<(String, String)>> = OnceLock::new();
const RECENT_CAP: usize = 4096;

fn recent_state() -> &'static Mutex<(String, String)> {
    RECENT.get_or_init(|| Mutex::new((String::new(), String::new())))
}

fn record_recent(category: &str, text: &str) {
    let mut guard = recent_state()
        .lock()
        .expect("recent output buffer poisoned");
    let buf = if category == "stderr" {
        &mut guard.1
    } else {
        &mut guard.0
    };
    buf.push_str(text);
    if buf.len() > RECENT_CAP {
        // Keep the tail, on a char boundary.
        let mut cut = buf.len() - RECENT_CAP;
        while !buf.is_char_boundary(cut) {
            cut += 1;
        }
        buf.drain(..cut);
    }
}

/// Tail of everything the process (externals, drains) recently wrote to the
/// given stream ("stdout"/"stderr").
pub(crate) fn recent_output(category: &str) -> String {
    let guard = recent_state()
        .lock()
        .expect("recent output buffer poisoned");
    if category == "stderr" {
        guard.1.clone()
    } else {
        guard.0.clone()
    }
}

pub(crate) struct OutputCapture {
    /// The real stdout, for the DAP protocol.
    pub(crate) dap_out: File,
    stdout_rx: PipeReader,
    stderr_rx: PipeReader,
}

pub(crate) fn detach_stdin() -> File {
    #[cfg(windows)]
    {
        use std::os::windows::io::{AsHandle, AsRawHandle, OwnedHandle};
        let stdin = std::io::stdin();
        let dup: OwnedHandle = stdin
            .as_handle()
            .try_clone_to_owned()
            .expect("duplicate stdin handle");
        if let Ok(nul) = File::open("NUL") {
            unsafe {
                windows_sys::Win32::System::Console::SetStdHandle(
                    windows_sys::Win32::System::Console::STD_INPUT_HANDLE,
                    nul.as_raw_handle() as _,
                );
            }
            // The handle installed via SetStdHandle must live forever.
            std::mem::forget(nul);
        }
        File::from(dup)
    }
    #[cfg(unix)]
    {
        use std::os::fd::{AsFd, AsRawFd, OwnedFd};
        let stdin = std::io::stdin();
        let dup: OwnedFd = stdin.as_fd().try_clone_to_owned().expect("dup stdin fd");
        if let Ok(null) = File::open("/dev/null") {
            let rc = unsafe { libc::dup2(null.as_raw_fd(), 0) };
            if rc == -1 {
                panic!(
                    "dup2(/dev/null, 0) failed: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
        File::from(dup)
    }
}

/// Swap process stdout/stderr for capture pipes; returns the real stdout
/// (for DAP) and the pipe read ends. Call `spawn_forwarders` once a
/// DapWriter exists.
pub(crate) fn install_output_capture() -> OutputCapture {
    let (stdout_rx, stdout_tx) = std::io::pipe().expect("stdout capture pipe");
    let (stderr_rx, stderr_tx) = std::io::pipe().expect("stderr capture pipe");

    #[cfg(windows)]
    let dap_out = {
        use std::os::windows::io::{AsHandle, IntoRawHandle, OwnedHandle};
        let stdout = std::io::stdout();
        let dup: OwnedHandle = stdout
            .as_handle()
            .try_clone_to_owned()
            .expect("duplicate stdout handle");
        unsafe {
            use windows_sys::Win32::System::Console::{
                STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
            };
            // into_raw_handle: the pipe write ends must live forever — they
            // are the process's std handles now.
            SetStdHandle(STD_OUTPUT_HANDLE, stdout_tx.into_raw_handle() as _);
            SetStdHandle(STD_ERROR_HANDLE, stderr_tx.into_raw_handle() as _);
        }
        File::from(dup)
    };
    #[cfg(unix)]
    let dap_out = {
        use std::os::fd::{AsFd, AsRawFd, OwnedFd};
        let stdout = std::io::stdout();
        let dup: OwnedFd = stdout.as_fd().try_clone_to_owned().expect("dup stdout fd");
        unsafe {
            libc::dup2(stdout_tx.as_raw_fd(), 1);
            libc::dup2(stderr_tx.as_raw_fd(), 2);
        }
        // fds 1/2 are dups; the originals may drop.
        drop(stdout_tx);
        drop(stderr_tx);
        File::from(dup)
    };

    OutputCapture {
        dap_out,
        stdout_rx,
        stderr_rx,
    }
}

/// Start the forwarder threads translating captured process output into DAP
/// `output` events.
pub(crate) fn spawn_forwarders(capture: OutputCapture, writer: &DapWriter) {
    forward(capture.stdout_rx, "stdout", writer.clone());
    forward(capture.stderr_rx, "stderr", writer.clone());
    // Only now is it safe for `flush_output` to push markers through the
    // process handles: they lead to the pipes, and someone is draining them.
    CAPTURING.store(true, Ordering::Release);
}

fn forward(mut rx: PipeReader, category: &'static str, writer: DapWriter) {
    std::thread::Builder::new()
        .name(format!("nu-{category}-fwd"))
        .spawn(move || {
            use std::io::Read;
            let mut buf = [0u8; 8192];
            // Carry-over so a MARKER split across reads is still found.
            let mut pending: Vec<u8> = Vec::new();
            loop {
                let n = match rx.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                pending.extend_from_slice(&buf[..n]);

                // Strip and count flush markers.
                let mut seen = 0u64;
                while let Some(at) = find(&pending, MARKER) {
                    pending.drain(at..at + MARKER.len());
                    seen += 1;
                }

                // Hold back a potential marker prefix at the tail.
                let keep = marker_prefix_len(&pending);
                let emit = pending.len() - keep;
                if emit > 0 {
                    let text = String::from_utf8_lossy(&pending[..emit]).to_string();
                    record_recent(category, &text);
                    writer.output(category, text);
                    pending.drain(..emit);
                }

                if seen > 0 {
                    let (count, cv) = flush_state();
                    *count.lock().expect("flush state") += seen;
                    cv.notify_all();
                }
            }
        })
        .expect("spawn output forwarder");
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Length of the longest strict MARKER prefix at the end of `data`.
fn marker_prefix_len(data: &[u8]) -> usize {
    let max = (MARKER.len() - 1).min(data.len());
    (1..=max)
        .rev()
        .find(|&l| data[data.len() - l..] == MARKER[..l])
        .unwrap_or(0)
}

/// Push a marker through both captured streams and wait until the
/// forwarders processed it — everything written before the marker has been
/// emitted as output events. Bounded by `timeout`.
///
/// No-op unless [`spawn_forwarders`] installed the capture: without it the
/// marker bytes would go to the host's real stdout/stderr (see [`crate::serve`],
/// which leaves process stdio untouched) and nothing would ever count them.
pub(crate) fn flush_output(timeout: Duration) {
    use std::io::Write;
    if !CAPTURING.load(Ordering::Acquire) {
        return;
    }
    let (count, cv) = flush_state();
    let target = { *count.lock().expect("flush state") + 2 };
    {
        // These go to the swapped process handles — i.e. the capture pipes.
        let _ = std::io::stdout().write_all(MARKER);
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().write_all(MARKER);
        let _ = std::io::stderr().flush();
    }

    let deadline = nu_utils::time::Instant::now() + timeout;
    let mut guard = count.lock().expect("flush state");
    while *guard < target {
        let left = deadline.saturating_duration_since(nu_utils::time::Instant::now());
        if left.is_zero() {
            break;
        }
        let (g, _) = cv.wait_timeout(guard, left).expect("flush state");
        guard = g;
    }
}
