use nu_utils::time::Instant;
use std::{
    io::{self, Write},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

/// How long a partial line waits for more output before [`JobOutput::drain`] takes it anyway.
///
/// Separate `print -n` calls get lines of their own, while the chunks of one stream, which
/// arrive back to back, stay together.
const PARTIAL_LINE_IDLE: Duration = Duration::from_millis(100);

/// What background jobs write to the terminal while the REPL holds it.
///
/// Output a background job writes straight to the terminal while the line editor reads a line
/// lands on top of the prompt, and the editor's next repaint erases it. While the REPL holds the
/// terminal, that output waits here instead, and the line editor prints it above the prompt. Jobs
/// write through a [`JobOutputWriter`].
///
/// Shared, like [`PromptState`](super::PromptState), with every job that clones the engine state.
#[derive(Debug, Default)]
pub struct JobOutput {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    /// `Some` while the REPL holds the terminal: the bytes queued so far, in arrival order.
    held: Option<Vec<u8>>,
    /// Background job threads still running. A killed job counts until its thread returns.
    running_jobs: usize,
    /// When output was last queued, to tell a finished partial line from one still being written.
    last_queued: Option<Instant>,
}

impl JobOutput {
    /// Create a `JobOutput` that is not holding the terminal.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // A job that panicked while queueing leaves the bytes intact; keep using them.
        self.state
            .lock()
            .unwrap_or_else(|poisoned_error| poisoned_error.into_inner())
    }

    /// Start queueing: the REPL is about to hand the terminal to the line editor.
    ///
    /// Waits for a job's write in progress, so it can't land once the line editor has the
    /// terminal. Returns whether this call started holding, so a caller nested in a hold, like
    /// `input` run from a keybinding, can leave its release to whoever started it.
    pub fn hold(&self) -> bool {
        let mut state = self.lock();
        let started = state.held.is_none();
        state.held.get_or_insert_with(Vec::new);
        started
    }

    /// Stop queueing and pass the output the line editor has not printed yet to `write`, now that
    /// the terminal is back in its normal mode.
    ///
    /// `write` runs under the lock and only when output is left, so a job writing at the same
    /// time waits and its output lands after this. A partial line at the end gets its newline, so
    /// what the shell prints next starts on a line of its own.
    pub fn release(&self, write: impl FnOnce(&[u8])) {
        let mut state = self.lock();
        if let Some(mut leftover) = state.held.take().filter(|leftover| !leftover.is_empty()) {
            if !leftover.ends_with(b"\n") {
                leftover.push(b'\n');
            }
            write(&leftover);
        }
    }

    /// Take the queued lines for the line editor to print, and keep queueing.
    ///
    /// Whoever prints it puts each drained piece on lines of its own, so a line still being
    /// written stays queued while more of it may follow: until its newline arrives, or no job
    /// thread is running, or no output was queued for [`PARTIAL_LINE_IDLE`].
    pub fn drain(&self) -> Option<String> {
        let mut state = self.lock();
        let jobs_running = state.running_jobs > 0;
        let partial_done = !jobs_running
            || state
                .last_queued
                .is_none_or(|queued| queued.elapsed() >= PARTIAL_LINE_IDLE);
        let held = state.held.as_mut()?;
        let line_start = held
            .iter()
            .rposition(|&byte| byte == b'\n')
            .map_or(0, |newline| newline + 1);
        let end = if partial_done {
            // Keep a character a stream has only written part of, since its job may finish it.
            match std::str::from_utf8(&held[line_start..]) {
                Err(err) if err.error_len().is_none() && jobs_running => {
                    line_start + err.valid_up_to()
                }
                _ => held.len(),
            }
        } else {
            line_start
        };
        if end == 0 {
            return None;
        }

        let rest = held.split_off(end);
        let lines = std::mem::replace(held, rest);
        Some(match String::from_utf8(lines) {
            Ok(lines) => lines,
            Err(err) => String::from_utf8_lossy(err.as_bytes()).into_owned(),
        })
    }

    /// Whether the line editor must keep checking for output while it waits for input: a job
    /// thread is running and may write, or output is queued.
    pub fn needs_polling(&self) -> bool {
        let state = self.lock();
        state.running_jobs > 0 || state.held.as_ref().is_some_and(|held| !held.is_empty())
    }

    /// Count a background job thread as running until the returned guard is dropped, so the line
    /// editor keeps checking for its output.
    ///
    /// Take it before the thread starts and drop it when the thread returns. `job kill` takes a
    /// job out of the job table while its thread may still write.
    #[must_use]
    pub fn job_running(self: &Arc<Self>) -> RunningJob {
        self.lock().running_jobs += 1;
        RunningJob(Arc::clone(self))
    }
}

/// Drop the start of the queued partial line that a later carriage return writes over.
///
/// A job that redraws one line with `\r`, like a progress bar, never finishes it, so the line
/// would wait in the queue and grow until the prompt is released. The terminal would only show
/// what follows the last `\r` anyway. A `\r` that is the last byte stays, since the next write
/// may make it part of a `\r\n` line ending.
fn discard_overwritten(held: &mut Vec<u8>) {
    let line_start = held
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |newline| newline + 1);
    let partial = &held[line_start..held.len().saturating_sub(1).max(line_start)];
    if let Some(carriage_return) = partial.iter().rposition(|&byte| byte == b'\r') {
        // `\r` is ASCII, so this never cuts a UTF-8 character in two.
        held.drain(line_start..line_start + carriage_return);
    }
}

/// A background job thread that [`JobOutput`] counts as running until this is dropped.
#[derive(Debug)]
pub struct RunningJob(Arc<JobOutput>);

impl Drop for RunningJob {
    fn drop(&mut self) {
        let mut state = self.0.lock();
        state.running_jobs = state.running_jobs.saturating_sub(1);
    }
}

/// Writes a thread's output to stdout or stderr. While the REPL holds the terminal, a background
/// job's output to it waits in [`JobOutput`] instead.
///
/// A background job writes under the [`JobOutput`] lock, so [`JobOutput::hold`] waits for a write
/// in progress and [`JobOutput::release`] writes the leftover before any later write.
pub struct JobOutputWriter<'a, W> {
    /// The queue, for a background job's output. `None` writes straight to `inner`.
    job_output: Option<&'a JobOutput>,
    inner: W,
    /// Whether `inner` reaches the terminal the line editor paints on.
    is_terminal: fn(&W) -> bool,
    /// `is_terminal`'s answer, asked only once the REPL holds the terminal during a write.
    reaches_terminal: Option<bool>,
}

impl<'a, W: Write> JobOutputWriter<'a, W> {
    /// Write to `inner`, queueing in `job_output` while the REPL holds the terminal and
    /// `is_terminal` says `inner` reaches it.
    pub fn new(job_output: Option<&'a JobOutput>, inner: W, is_terminal: fn(&W) -> bool) -> Self {
        Self {
            job_output,
            inner,
            is_terminal,
            reaches_terminal: None,
        }
    }

    fn reaches_terminal(&mut self) -> bool {
        *self
            .reaches_terminal
            .get_or_insert_with(|| (self.is_terminal)(&self.inner))
    }
}

impl<W: Write> Write for JobOutputWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let Some(job_output) = self.job_output else {
            return self.inner.write(buf);
        };
        let mut state = job_output.lock();
        let state = &mut *state;
        match state.held.as_mut() {
            Some(held) if self.reaches_terminal() => {
                held.extend_from_slice(buf);
                discard_overwritten(held);
                state.last_queued = Some(Instant::now());
            }
            // Write and flush under the lock, so `hold` can't hand the terminal to the line
            // editor halfway through.
            _ => {
                self.inner.write_all(buf)?;
                self.inner.flush()?;
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(output: &JobOutput, reaches_terminal: bool, bytes: &[u8]) -> Vec<u8> {
        let mut written = Vec::new();
        JobOutputWriter::new(
            Some(output),
            &mut written,
            if reaches_terminal {
                |_| true
            } else {
                |_| false
            },
        )
        .write_all(bytes)
        .expect("writing to a Vec");
        written
    }

    #[test]
    fn job_output_is_written_while_nothing_holds_the_terminal() {
        let output = JobOutput::new();

        assert_eq!(write(&output, true, b"boom\n"), b"boom\n");
        assert!(!output.needs_polling());
    }

    #[test]
    fn job_output_to_the_terminal_is_queued_while_held() {
        let output = JobOutput::new();
        output.hold();

        assert_eq!(write(&output, true, b"one\n"), b"");
        assert_eq!(write(&output, true, b"two\n"), b"");

        assert_eq!(output.drain().as_deref(), Some("one\ntwo\n"));
        assert_eq!(output.drain(), None);
    }

    #[test]
    fn job_output_elsewhere_is_written_while_held() {
        let output = JobOutput::new();
        output.hold();

        assert_eq!(write(&output, false, b"to a file\n"), b"to a file\n");
        assert_eq!(output.drain(), None);
    }

    #[test]
    fn output_without_a_queue_is_written_while_held() {
        let output = JobOutput::new();
        output.hold();
        let mut written = Vec::new();

        JobOutputWriter::new(None, &mut written, |_| true)
            .write_all(b"main\n")
            .expect("writing to a Vec");

        assert_eq!(written, b"main\n");
        assert_eq!(output.drain(), None);
    }

    /// Fails the test unless the [`JobOutput`] lock is held while it writes.
    struct AssertLocked<'a>(&'a JobOutput);

    impl Write for AssertLocked<'_> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            assert!(self.0.state.try_lock().is_err(), "written without the lock");
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn job_output_is_written_under_the_lock() {
        let output = JobOutput::new();

        JobOutputWriter::new(Some(&output), AssertLocked(&output), |_| true)
            .write_all(b"direct\n")
            .expect("writing");
    }

    #[test]
    fn one_writer_follows_hold_and_release() {
        let output = JobOutput::new();
        let mut written = Vec::new();
        let mut leftover = Vec::new();

        {
            let mut writer = JobOutputWriter::new(Some(&output), &mut written, |_| true);
            writer.write_all(b"before\n").expect("writing");
            output.hold();
            writer.write_all(b"held\n").expect("writing");
            output.release(|rest| leftover.extend_from_slice(rest));
            writer.write_all(b"after\n").expect("writing");
        }

        assert_eq!(leftover, b"held\n");
        assert_eq!(written, b"before\nafter\n");
    }

    #[test]
    fn a_partial_line_waits_for_its_newline_while_a_job_runs() {
        let output = Arc::new(JobOutput::new());
        let _job = output.job_running();
        output.hold();

        write(&output, true, b"1 ");
        write(&output, true, b"2 ");
        assert_eq!(output.drain(), None);

        write(&output, true, b"3\n4");
        assert_eq!(output.drain().as_deref(), Some("1 2 3\n"));
        write(&output, true, b"\n");
        assert_eq!(output.drain().as_deref(), Some("4\n"));
    }

    #[test]
    fn a_partial_line_is_drained_once_no_job_runs() {
        let output = Arc::new(JobOutput::new());
        let job = output.job_running();
        output.hold();
        write(&output, true, b"no newline");
        assert_eq!(output.drain(), None);

        drop(job);

        assert_eq!(output.drain().as_deref(), Some("no newline"));
    }

    #[test]
    fn a_character_split_across_writes_is_drained_whole() {
        let output = Arc::new(JobOutput::new());
        let _job = output.job_running();
        output.hold();
        let bytes = "é\n".as_bytes();

        write(&output, true, &bytes[..1]);
        write(&output, true, &bytes[1..]);

        assert_eq!(output.drain().as_deref(), Some("é\n"));
    }

    #[test]
    fn polling_is_needed_while_a_job_runs_or_output_waits() {
        let output = Arc::new(JobOutput::new());
        output.hold();
        assert!(!output.needs_polling());

        let job = output.job_running();
        assert!(output.needs_polling());
        write(&output, true, b"last words\n");
        drop(job);
        assert!(
            output.needs_polling(),
            "queued output still has to be drained"
        );

        output.drain();
        assert!(!output.needs_polling());
    }

    #[test]
    fn release_writes_undrained_output_and_stops_queueing() {
        let output = JobOutput::new();
        output.hold();
        write(&output, true, b"left over");
        let mut written = Vec::new();

        output.release(|leftover| written.extend_from_slice(leftover));

        assert_eq!(written, b"left over\n");
        assert_eq!(write(&output, true, b"after\n"), b"after\n");
    }

    /// Make the queued output look as if nothing was written for [`PARTIAL_LINE_IDLE`].
    fn go_idle(output: &JobOutput) {
        output.lock().last_queued = Instant::now().checked_sub(PARTIAL_LINE_IDLE);
    }

    #[test]
    fn an_idle_partial_line_is_drained_while_a_job_runs() {
        let output = Arc::new(JobOutput::new());
        let _job = output.job_running();
        output.hold();
        write(&output, true, b"done\n1 ");
        assert_eq!(output.drain().as_deref(), Some("done\n"));

        go_idle(&output);

        assert_eq!(output.drain().as_deref(), Some("1 "));
        assert_eq!(output.drain(), None);
    }

    #[test]
    fn an_idle_partial_line_keeps_a_character_cut_in_two() {
        let output = Arc::new(JobOutput::new());
        let _job = output.job_running();
        output.hold();
        let bytes = "aé".as_bytes();
        write(&output, true, &bytes[..2]);

        go_idle(&output);
        assert_eq!(output.drain().as_deref(), Some("a"));

        write(&output, true, &bytes[2..]);
        go_idle(&output);
        assert_eq!(output.drain().as_deref(), Some("é"));
    }

    #[test]
    fn hold_reports_whether_it_started_holding() {
        let output = JobOutput::new();

        assert!(output.hold());
        assert!(!output.hold());
        output.release(|_| ());
        assert!(output.hold());
    }

    #[test]
    fn release_keeps_a_finished_line_as_is() {
        let output = JobOutput::new();
        output.hold();
        write(&output, true, b"done\n");
        let mut written = Vec::new();

        output.release(|leftover| written.extend_from_slice(leftover));

        assert_eq!(written, b"done\n");
    }

    #[test]
    fn a_redrawn_partial_line_keeps_only_its_last_version() {
        let output = Arc::new(JobOutput::new());
        let _job = output.job_running();
        output.hold();

        write(&output, true, b"done\n\r10%");
        write(&output, true, b"\r20%");
        write(&output, true, b"\r30%");

        assert_eq!(output.drain().as_deref(), Some("done\n"));
        let mut written = Vec::new();
        output.release(|leftover| written.extend_from_slice(leftover));
        assert_eq!(written, b"\r30%\n");
    }

    #[test]
    fn a_trailing_carriage_return_can_still_end_the_line() {
        let output = Arc::new(JobOutput::new());
        let _job = output.job_running();
        output.hold();

        write(&output, true, b"kept\r");
        write(&output, true, b"\n");

        assert_eq!(output.drain().as_deref(), Some("kept\r\n"));
    }

    #[test]
    fn release_skips_the_writer_when_nothing_is_left() {
        let output = JobOutput::new();
        output.hold();
        write(&output, true, b"printed\n");
        output.drain();
        let mut calls = 0;

        output.release(|_| calls += 1);
        output.release(|_| calls += 1);

        assert_eq!(calls, 0);
    }

    #[test]
    fn a_job_writing_during_release_lands_after_the_leftover() {
        let output = JobOutput::new();
        output.hold();
        write(&output, true, b"first\n");
        let written = Mutex::new(Vec::new());

        // Scoped threads may borrow `output` and `written`; the scope joins them before it ends.
        std::thread::scope(|scope| {
            output.release(|leftover| {
                // A job writes while the leftover is being written. Its write waits for `release`
                // to let go of the lock.
                scope.spawn(|| {
                    let direct = write(&output, true, b"second\n");
                    written.lock().expect("written lock").extend(direct);
                });
                std::thread::sleep(std::time::Duration::from_millis(50));
                written.lock().expect("written lock").extend(leftover);
            });
        });

        assert_eq!(*written.lock().expect("written lock"), b"first\nsecond\n");
    }

    #[test]
    fn hold_twice_keeps_queued_output() {
        let output = JobOutput::new();
        output.hold();
        write(&output, true, b"kept\n");

        output.hold();

        assert_eq!(output.drain().as_deref(), Some("kept\n"));
    }

    #[test]
    fn running_jobs_are_counted_until_their_guards_drop() {
        let output = Arc::new(JobOutput::new());

        let first = output.job_running();
        let second = output.job_running();
        drop(first);
        assert!(output.needs_polling());

        drop(second);
        assert!(!output.needs_polling());
    }
}
