//! Process stdio plumbing.
//!
//! This process's stdin/stdout carry the DAP protocol, but an embedded nu
//! evaluation writes to process stdout from places no stack redirection
//! reaches (drains print byte streams directly, externals inherit process
//! handles, stray `eprintln!`s), and an external *reading* inherited stdin
//! would steal DAP frames or block forever.
//!
//! Fix, applied once at startup:
//! - stdin: duplicate the real handle for DAP reading, then point
//!   process-level stdin at the null device — children see immediate EOF.
//! - stdout/stderr: duplicate the real stdout for DAP writing, then point
//!   process-level stdout/stderr at capture pipes whose forwarder threads
//!   re-emit everything as DAP `output` events.
//!
//! The pipe write ends live forever (they *are* the std handles now), so the
//! forwarders never see EOF. `flush_output` instead pushes a marker through
//! and waits for it — call it before `terminated` so late output isn't lost.

use parking_lot::{Condvar, Mutex};
use std::fs::File;
use std::io::{PipeReader, Read, Write};
#[cfg(unix)]
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
#[cfg(windows)]
use std::os::windows::io::{AsHandle, AsRawHandle, IntoRawHandle, OwnedHandle};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
#[cfg(windows)]
use windows_sys::Win32::System::Console::{
    STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
};

use crate::dap::protocol::DapWriter;

const MARKER: &[u8] = b"\x01<NU-DAP-FLUSH>\x01";

enum PipeType {
    StdOut,
    StdErr,
}

impl PipeType {
    /// Also the DAP `output` event category for this stream.
    fn as_str(&self) -> &'static str {
        match self {
            PipeType::StdOut => "stdout",
            PipeType::StdErr => "stderr",
        }
    }
}

impl std::fmt::Display for PipeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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

fn record_recent(pipe_type: &PipeType, text: &str) {
    let mut guard = recent_state().lock();

    let buffer = match pipe_type {
        PipeType::StdErr => &mut guard.1,
        PipeType::StdOut => &mut guard.0,
    };

    buffer.push_str(text);

    if buffer.len() > RECENT_CAP {
        // Keep the tail, on a char boundary.
        let mut cut = buffer.len() - RECENT_CAP;
        while !buffer.is_char_boundary(cut) {
            cut += 1;
        }

        buffer.drain(..cut);
    }
}

/// Tail of everything the process (externals, drains) recently wrote to the
/// given stream ("stdout"/"stderr").
pub(crate) fn recent_output(category: &str) -> String {
    let guard = recent_state().lock();
    if category == "stderr" {
        guard.1.clone()
    } else {
        guard.0.clone()
    }
}

pub(crate) struct OutputCapture {
    /// The real stdout, for the DAP protocol.
    pub dap_out: File,
    stdout_rx: PipeReader,
    stderr_rx: PipeReader,
}

pub(crate) fn detach_stdin() -> File {
    #[cfg(windows)]
    {
        let stdin = std::io::stdin();
        let dup: OwnedHandle = stdin
            .as_handle()
            .try_clone_to_owned()
            .expect("duplicate stdin handle");
        if let Ok(nul) = File::open("NUL") {
            unsafe {
                SetStdHandle(STD_INPUT_HANDLE, nul.as_raw_handle() as _);
            }
            // The handle installed via SetStdHandle must live forever.
            std::mem::forget(nul);
        }
        File::from(dup)
    }
    #[cfg(unix)]
    {
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
        let stdout = std::io::stdout();
        let dup: OwnedHandle = stdout
            .as_handle()
            .try_clone_to_owned()
            .expect("duplicate stdout handle");
        unsafe {
            // into_raw_handle: the pipe write ends must live forever — they
            // are the process's std handles now.
            SetStdHandle(STD_OUTPUT_HANDLE, stdout_tx.into_raw_handle() as _);
            SetStdHandle(STD_ERROR_HANDLE, stderr_tx.into_raw_handle() as _);
        }
        File::from(dup)
    };
    #[cfg(unix)]
    let dap_out = {
        let stdout = std::io::stdout();
        let dup: OwnedFd = stdout.as_fd().try_clone_to_owned().expect("dup stdout fd");
        // Checked, like `detach_stdin`: a silent failure here would leave the
        // script writing straight onto the DAP wire and corrupt the protocol.
        for (from, to) in [(stdout_tx.as_raw_fd(), 1), (stderr_tx.as_raw_fd(), 2)] {
            if unsafe { libc::dup2(from, to) } == -1 {
                panic!(
                    "dup2(capture pipe, {to}) failed: {}",
                    std::io::Error::last_os_error()
                );
            }
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
    forward(capture.stdout_rx, PipeType::StdOut, writer.clone());
    forward(capture.stderr_rx, PipeType::StdErr, writer.clone());

    // Only now is it safe for `flush_output` to push markers through the
    // process handles: they lead to the pipes, and someone is draining them.
    CAPTURING.store(true, Ordering::Release);
}

fn forward(mut rx: PipeReader, pipe_type: PipeType, writer: DapWriter) {
    std::thread::Builder::new()
        .name(format!("nu-{pipe_type}-fwd"))
        .spawn(move || {
            let mut buf = [0u8; 8192];
            // Carry-over, so a MARKER split across reads is still found.
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

                // Hold back a potential marker prefix at the tail, and any
                // incomplete UTF-8 sequence behind it: a read boundary can
                // land mid-codepoint, and emitting that half would decode to
                // a replacement character that never resolves.
                let keep = marker_prefix_len(&pending);
                let body = pending.len() - keep;
                let emit = body - utf8_tail_len(&pending[..body]);
                if emit > 0 {
                    let text = String::from_utf8_lossy(&pending[..emit]).to_string();
                    record_recent(&pipe_type, &text);
                    writer.output(pipe_type.as_str(), text);
                    pending.drain(..emit);
                }

                if seen > 0 {
                    let (count, cv) = flush_state();
                    *count.lock() += seen;
                    cv.notify_all();
                }
            }
        })
        .expect("spawn output forwarder");
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Length of the trailing bytes of `data` that form an incomplete UTF-8
/// sequence, and so must wait for the next read. Zero when `data` ends on a
/// character boundary (the common case) or ends in bytes that can never start
/// a valid sequence, which are better decoded lossily now than held forever.
fn utf8_tail_len(data: &[u8]) -> usize {
    // A sequence is at most 4 bytes, so at most 3 can be pending.
    for back in 1..=3.min(data.len()) {
        let byte = data[data.len() - back];
        // Continuation byte (10xxxxxx): keep walking to the lead byte.
        if byte & 0b1100_0000 == 0b1000_0000 {
            continue;
        }
        // Lead byte: `back` bytes are present, `needed` are required.
        let needed = match byte {
            0x00..=0x7F => 1,
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF7 => 4,
            // Not a lead byte at all: nothing to wait for.
            _ => return 0,
        };
        return if needed > back { back } else { 0 };
    }
    0
}

/// Length of the longest strict MARKER prefix at the end of `data`.
fn marker_prefix_len(data: &[u8]) -> usize {
    let max = (MARKER.len() - 1).min(data.len());
    (1..=max)
        .rev()
        .find(|&l| data[data.len() - l..] == MARKER[..l])
        .unwrap_or(0)
}

/// Push a marker through both captured streams and wait, up to `timeout`,
/// until the forwarders process it — by then everything written before it has
/// been emitted as output events.
///
/// No-op unless [`spawn_forwarders`] installed the capture; without it the
/// markers would land on the host's real stdout/stderr uncounted (see
/// [`crate::serve`], which leaves process stdio untouched).
pub(crate) fn flush_output(timeout: Duration) {
    if !CAPTURING.load(Ordering::Acquire) {
        return;
    }
    let (count, cv) = flush_state();
    let target = { *count.lock() + 2 };
    {
        // These go to the swapped handles — i.e. the capture pipes.
        let _ = std::io::stdout().write_all(MARKER);
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().write_all(MARKER);
        let _ = std::io::stderr().flush();
    }

    let deadline = nu_utils::time::Instant::now() + timeout;
    let mut guard = count.lock();
    while *guard < target {
        let left = deadline.saturating_duration_since(nu_utils::time::Instant::now());
        if left.is_zero() {
            break;
        }
        cv.wait_for(&mut guard, left);
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`crate::stdio`].

    use super::{MARKER, marker_prefix_len, utf8_tail_len};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// The forwarder reads fixed-size chunks, so a multi-byte character can
    /// straddle two reads. Whatever is held back must be exactly the
    /// incomplete tail.
    #[rstest]
    #[case::empty(&[], 0)]
    #[case::ascii(b"hello", 0)]
    #[case::complete_two_byte("é".as_bytes(), 0)]
    #[case::complete_four_byte("🦀".as_bytes(), 0)]
    #[case::split_two_byte(&[0xC3], 1)]
    #[case::split_three_byte(&[0xE2, 0x82], 2)]
    #[case::split_four_byte(&[0xF0, 0x9F, 0xA6], 3)]
    #[case::text_then_split_char(&[b'o', b'k', b' ', 0xF0, 0x9F], 2)]
    // A stray continuation byte with no lead in reach can never complete.
    #[case::orphan_continuation(&[0x80, 0x80, 0x80], 0)]
    fn incomplete_utf8_tails_are_held_back(#[case] data: &[u8], #[case] expected: usize) {
        assert_eq!(utf8_tail_len(data), expected);
    }

    /// Reassembling across a boundary must lose nothing: the held-back tail
    /// plus the next read decodes to the original text.
    #[test]
    fn a_character_split_across_reads_survives() {
        let text = "héllo 🦀!";
        let bytes = text.as_bytes();
        let mut out = String::new();
        // Every possible split point, to catch an off-by-one in either half.
        for at in 0..=bytes.len() {
            let (head, tail) = bytes.split_at(at);
            let keep = utf8_tail_len(head);
            let emit = head.len() - keep;
            out.clear();
            out.push_str(&String::from_utf8_lossy(&head[..emit]));
            out.push_str(&String::from_utf8_lossy(&[&head[emit..], tail].concat()));
            assert_eq!(out, text, "split at {at}");
        }
    }

    /// The marker holdback this mirrors: a marker split across reads is still
    /// recognised, so its bytes never reach the client as output.
    #[rstest]
    #[case::none(b"plain text", 0)]
    #[case::one_byte(&[b'o', b'u', b't', 0x01], 1)]
    #[case::partial(&[b'o', b'u', b't', 0x01, b'<', b'N', b'U'], 4)]
    // MARKER opens and closes with the same byte, so its own last byte is a
    // 1-byte prefix. Harmless: `find` strips whole markers before this runs.
    #[case::whole_marker_keeps_its_trailing_byte(MARKER, 1)]
    fn marker_prefixes_at_the_tail_are_held_back(#[case] data: &[u8], #[case] expected: usize) {
        assert_eq!(marker_prefix_len(data), expected);
    }
}
