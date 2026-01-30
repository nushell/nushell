use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::Print,
    terminal::{self, ClearType},
};
use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::{self, io::IoError};

use std::{io::Write, time::Duration};

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
        if let Some(prompt) = &prompt {
            match &default_val {
                None => print!("{prompt}"),
                Some(val) => print!("{prompt} (default: {val})"),
            }
            let _ = std::io::stdout().flush();
        }

        let mut buf = String::new();
        let mut cursor_pos: usize = 0;

        crossterm::terminal::enable_raw_mode().map_err(&from_io_error)?;
        // clear terminal events
        while crossterm::event::poll(Duration::from_secs(0)).map_err(&from_io_error)? {
            // If there's an event, read it to remove it from the queue
            let _ = crossterm::event::read().map_err(&from_io_error)?;
        }

        loop {
            if i64::try_from(buf.chars().count()).unwrap_or(0) >= numchar.item {
                break;
            }
            match crossterm::event::read() {
                Ok(Event::Key(k)) => match k.kind {
                    KeyEventKind::Press | KeyEventKind::Repeat => {
                        match k.code {
                            KeyCode::Char(c) if k.modifiers == KeyModifiers::CONTROL => {
                                match c {
                                    'c' => {
                                        crossterm::terminal::disable_raw_mode()
                                            .map_err(&from_io_error)?;
                                        return Err(IoError::new(
                                            shell_error::io::ErrorKind::from_std(
                                                std::io::ErrorKind::Interrupted,
                                            ),
                                            call.head,
                                            None,
                                        )
                                        .into());
                                    }
                                    // Emacs keybindings
                                    'a' => cursor_pos = 0,
                                    'e' => cursor_pos = buf.chars().count(),
                                    'b' => {
                                        cursor_pos = cursor_pos.saturating_sub(1);
                                    }
                                    'f' => {
                                        if cursor_pos < buf.chars().count() {
                                            cursor_pos += 1;
                                        }
                                    }
                                    'j' => break, // Same as Enter
                                    'u' => {
                                        // Kill line before cursor
                                        let byte_pos = buf
                                            .char_indices()
                                            .nth(cursor_pos)
                                            .map(|(i, _)| i)
                                            .unwrap_or(buf.len());
                                        buf = buf[byte_pos..].to_string();
                                        cursor_pos = 0;
                                    }
                                    'k' => {
                                        // Kill line after cursor
                                        let byte_pos = buf
                                            .char_indices()
                                            .nth(cursor_pos)
                                            .map(|(i, _)| i)
                                            .unwrap_or(buf.len());
                                        buf.truncate(byte_pos);
                                    }
                                    'd' => {
                                        // Delete character at cursor
                                        if cursor_pos < buf.chars().count() {
                                            let byte_pos = buf
                                                .char_indices()
                                                .nth(cursor_pos)
                                                .map(|(i, _)| i)
                                                .unwrap_or(buf.len());
                                            if byte_pos < buf.len() {
                                                buf.remove(byte_pos);
                                            }
                                        }
                                    }
                                    'h' => {
                                        // Backward delete (same as Backspace)
                                        if cursor_pos > 0 {
                                            cursor_pos -= 1;
                                            let byte_pos = buf
                                                .char_indices()
                                                .nth(cursor_pos)
                                                .map(|(i, _)| i)
                                                .unwrap_or(buf.len());
                                            if byte_pos < buf.len() {
                                                buf.remove(byte_pos);
                                            }
                                        }
                                    }
                                    _ => continue,
                                }
                            }
                            KeyCode::Char(c) => {
                                if k.modifiers == KeyModifiers::ALT {
                                    continue;
                                }

                                if let Some(bytes_until) = bytes_until.as_ref()
                                    && bytes_until.bytes().contains(&(c as u8))
                                {
                                    break;
                                }
                                let byte_pos = buf
                                    .char_indices()
                                    .nth(cursor_pos)
                                    .map(|(i, _)| i)
                                    .unwrap_or(buf.len());
                                buf.insert(byte_pos, c);
                                cursor_pos += 1;
                            }
                            KeyCode::Backspace => {
                                if cursor_pos > 0 {
                                    cursor_pos -= 1;
                                    let byte_pos = buf
                                        .char_indices()
                                        .nth(cursor_pos)
                                        .map(|(i, _)| i)
                                        .unwrap_or(buf.len());
                                    if byte_pos < buf.len() {
                                        buf.remove(byte_pos);
                                    }
                                }
                            }
                            KeyCode::Delete => {
                                if cursor_pos < buf.chars().count() {
                                    let byte_pos = buf
                                        .char_indices()
                                        .nth(cursor_pos)
                                        .map(|(i, _)| i)
                                        .unwrap_or(buf.len());
                                    if byte_pos < buf.len() {
                                        buf.remove(byte_pos);
                                    }
                                }
                            }
                            KeyCode::Left => {
                                cursor_pos = cursor_pos.saturating_sub(1);
                            }
                            KeyCode::Right => {
                                if cursor_pos < buf.chars().count() {
                                    cursor_pos += 1;
                                }
                            }
                            KeyCode::Home => cursor_pos = 0,
                            KeyCode::End => cursor_pos = buf.chars().count(),
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
                    crossterm::terminal::disable_raw_mode().map_err(&from_io_error)?;
                    return Err(from_io_error(event_error).into());
                }
            }
            if !suppress_output {
                // clear the current line and print the current buffer
                execute!(
                    std::io::stdout(),
                    terminal::Clear(ClearType::CurrentLine),
                    cursor::MoveToColumn(0),
                )
                .map_err(|err| IoError::new(err, call.head, None))?;
                if let Some(prompt) = &prompt {
                    execute!(std::io::stdout(), Print(prompt.to_string()))
                        .map_err(&from_io_error)?;
                }
                execute!(std::io::stdout(), Print(buf.to_string())).map_err(&from_io_error)?;
                // Position cursor correctly
                let prompt_len = prompt.as_ref().map(|p| p.chars().count()).unwrap_or(0);
                let cursor_col = prompt_len + cursor_pos;
                execute!(
                    std::io::stdout(),
                    cursor::MoveToColumn(cursor_col as u16),
                )
                .map_err(&from_io_error)?;
            }
        }
        crossterm::terminal::disable_raw_mode().map_err(&from_io_error)?;
        std::io::stdout().write_all(b"\n").map_err(&from_io_error)?;
        match default_val {
            Some(val) if buf.is_empty() => Ok(Value::string(val, call.head).into_pipeline_data()),
            _ => Ok(Value::string(buf, call.head).into_pipeline_data()),
        }
    }
}
