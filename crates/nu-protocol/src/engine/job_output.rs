use std::{
    io::{self, Write},
    sync::{Arc, Mutex, MutexGuard},
};

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
    /// terminal.
    pub fn hold(&self) {
        self.lock().held.get_or_insert_with(Vec::new);
    }

    /// Stop queueing and pass the output the line editor has not printed yet to `write`, now that
    /// the terminal is back in its normal mode.
    ///
    /// `write` runs under the lock and only when output is left, so a job writing at the same
    /// time waits and its output lands after this.
    pub fn release(&self, write: impl FnOnce(&[u8])) {
        let mut state = self.lock();
        if let Some(leftover) = state.held.take().filter(|leftover| !leftover.is_empty()) {
            write(&leftover);
        }
    }

    /// Take the queued lines for the line editor to print, and keep queueing.
    ///
    /// The line editor prints what it gets as whole lines, so a line still being written stays
    /// queued until its newline arrives. Once no job thread is running nothing can finish it, and
    /// it is taken too.
    pub fn drain(&self) -> Option<String> {
        let mut state = self.lock();
        let jobs_running = state.running_jobs > 0;
        let held = state.held.as_mut()?;
        let end = if jobs_running {
            held.iter()
                .rposition(|&byte| byte == b'\n')
                .map_or(0, |newline| newline + 1)
        } else {
            held.len()
        };
        if end == 0 {
            return None;
        }

        let rest = held.split_off(end);
        // Splitting after a newline never cuts a UTF-8 character in two.
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
        match state.held.as_mut() {
            Some(held) if self.reaches_terminal() => held.extend_from_slice(buf),
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

        assert_eq!(written, b"left over");
        assert_eq!(write(&output, true, b"after\n"), b"after\n");
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
