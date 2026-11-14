use crate::platform::RawModeGuard;
use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::Print,
    terminal::{self, ClearType},
};
use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::{
    engine::JobOutput,
    shell_error::{self, io::IoError},
};

use std::{io::Write, time::Duration};

/// How often `input` checks for background job output while it waits for a key.
const JOB_OUTPUT_POLL: Duration = Duration::from_millis(50);

/// Holds background job output while `input` reads a line, so it can print it above the line
/// instead of the job writing into it. On drop it writes what is left, after the line.
///
/// Holds nothing when someone already holds the job output, like the REPL when a keybinding
/// runs `input`, or when `input` runs in a background job itself.
struct HeldJobOutput<'a>(Option<&'a JobOutput>);

impl<'a> HeldJobOutput<'a> {
    fn hold(engine_state: &'a EngineState) -> Self {
        let job_output = &*engine_state.job_output;
        let held = !engine_state.is_background_job() && job_output.hold();
        if held {
            // The prompt goes after a job's unfinished line, not into it.
            job_output.end_job_line(|newline| {
                let _ = std::io::stdout().write_all(newline);
            });
        }
        Self(held.then_some(job_output))
    }
}

impl Drop for HeldJobOutput<'_> {
    fn drop(&mut self) {
        if let Some(job_output) = self.0 {
            job_output.release(|leftover| {
                let mut stderr = std::io::stderr();
                let _ = stderr.write_all(leftover);
                let _ = stderr.flush();
            });
        }
    }
}

/// Clear the line `input` reads on and draw `prompt` and `buf` there.
fn redraw_line(prompt: Option<&str>, buf: &str) -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        terminal::Clear(ClearType::CurrentLine),
        cursor::MoveToColumn(0),
    )?;
    if let Some(prompt) = prompt {
        execute!(stdout, Print(prompt))?;
    }
    execute!(stdout, Print(buf))
}

/// Print background job output where the line `input` reads on is, then draw that line again
/// below it.
fn print_above_line(
    output: &str,
    prompt: Option<&str>,
    buf: &str,
    suppress_output: bool,
) -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        terminal::Clear(ClearType::CurrentLine),
        cursor::MoveToColumn(0),
        Print(output),
    )?;
    if !output.ends_with('\n') {
        execute!(stdout, Print('\n'))?;
    }
    redraw_line(prompt, if suppress_output { "" } else { buf })
}

pub trait LegacyInput {
    fn legacy_input(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let prompt: Option<String> = call.opt(engine_state, stack, 0)?;
        let bytes_until: Option<String> = call.get_flag(engine_state, stack, "bytes-until-any")?;
        let suppress_output = call.has_flag(engine_state, stack, "suppress-output")?;
        let numchar: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "numchar")?;
        let numchar: Spanned<i64> = numchar.unwrap_or(Spanned {
            item: i64::MAX,
            span: call.head,
        });

        let from_io_error = IoError::factory(call.head, None);

        if numchar.item < 1 {
            return Err(ShellError::UnsupportedInput {
                msg: "Number of characters to read has to be positive".to_string(),
                input: "value originated from here".to_string(),
                msg_span: call.head,
                input_span: numchar.span,
            });
        }

        let default_val: Option<String> = call.get_flag(engine_state, stack, "default")?;

        // Acquire the guard (and its `require_stdin` check) before writing anything, so a
        // detached stack (completion worker or MCP) errors out before the prompt is printed.
        let raw_mode = RawModeGuard::acquire(stack, call.head)?;
        let job_output = HeldJobOutput::hold(engine_state);

        if let Some(prompt) = &prompt {
            match &default_val {
                None => print!("{prompt}"),
                Some(val) => print!("{prompt} (default: {val})"),
            }
            let _ = std::io::stdout().flush();
        }

        let mut buf = String::new();

        // clear terminal events
        while crossterm::event::poll(Duration::from_secs(0)).map_err(&from_io_error)? {
            // If there's an event, read it to remove it from the queue
            let _ = crossterm::event::read().map_err(&from_io_error)?;
        }

        loop {
            if i64::try_from(buf.len()).unwrap_or(0) >= numchar.item {
                break;
            }
            // While a job may write, wait for a key in short steps and print what jobs queued.
            if let Some(job_output) = job_output.0
                && job_output.needs_polling()
                && !crossterm::event::poll(JOB_OUTPUT_POLL).map_err(&from_io_error)?
            {
                if let Some(output) = job_output.drain() {
                    print_above_line(&output, prompt.as_deref(), &buf, suppress_output)
                        .map_err(&from_io_error)?;
                }
                continue;
            }
            match crossterm::event::read() {
                Ok(Event::Key(k)) => match k.kind {
                    KeyEventKind::Press | KeyEventKind::Repeat => {
                        match k.code {
                            // TODO: maintain keycode parity with existing command
                            KeyCode::Char(c) => {
                                if k.modifiers == KeyModifiers::ALT
                                    || k.modifiers == KeyModifiers::CONTROL
                                {
                                    if k.modifiers == KeyModifiers::CONTROL && c == 'c' {
                                        // End the line as Enter does, so the error report
                                        // starts on a line of its own.
                                        drop(raw_mode);
                                        let _ = std::io::stdout().write_all(b"\n");
                                        return Err(IoError::new(
                                            shell_error::io::ErrorKind::from_std(
                                                std::io::ErrorKind::Interrupted,
                                            ),
                                            call.head,
                                            None,
                                        )
                                        .into());
                                    }
                                    continue;
                                }

                                if let Some(bytes_until) = bytes_until.as_ref()
                                    && bytes_until.bytes().contains(&(c as u8))
                                {
                                    break;
                                }
                                buf.push(c);
                            }
                            KeyCode::Backspace => {
                                let _ = buf.pop();
                            }
                            KeyCode::Enter => {
                                break;
                            }
                            _ => continue,
                        }
                    }
                    _ => continue,
                },
                Ok(_) => continue,
                Err(event_error) => {
                    return Err(from_io_error(event_error).into());
                }
            }
            if !suppress_output {
                redraw_line(prompt.as_deref(), &buf).map_err(&from_io_error)?;
            }
        }
        // Leave raw mode before the trailing newline so it gets the usual carriage return.
        drop(raw_mode);
        std::io::stdout().write_all(b"\n").map_err(&from_io_error)?;
        match default_val {
            Some(val) if buf.is_empty() => Ok(Value::string(val, call.head).into_pipeline_data()),
            _ => Ok(Value::string(buf, call.head).into_pipeline_data()),
        }
    }
}
