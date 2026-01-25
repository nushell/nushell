use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{
    error::Error,
    io::{self, Write},
};

pub fn try_interaction(
    interactive: bool,
    prompt: String,
) -> (Result<Option<bool>, Box<dyn Error>>, bool) {
    let interaction = if interactive {
        match get_interactive_confirmation(prompt) {
            Ok(i) => Ok(Some(i)),
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    };

    let confirmed = match interaction {
        Ok(maybe_input) => maybe_input.unwrap_or(false),
        Err(_) => false,
    };

    (interaction, confirmed)
}

fn get_interactive_confirmation(prompt: String) -> Result<bool, Box<dyn Error>> {
    let mut stderr = io::stderr();

    // Print prompt
    eprint!("{} [Y/N]: ", prompt);
    stderr.flush()?;

    enable_raw_mode()?;
    scopeguard::defer! {
        let _ = disable_raw_mode();
        let _ = execute!(io::stderr(), Show);
    }

    let mut input = String::new();

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                // Handle Ctrl+C
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('c')
                {
                    eprint!("\r\n");
                    return Ok(false);
                }

                match key_event.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        eprint!("y\r\n");
                        return Ok(true);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        eprint!("n\r\n");
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        // Validate current input
                        if input.eq_ignore_ascii_case("y") {
                            eprint!("\r\n");
                            return Ok(true);
                        } else if input.eq_ignore_ascii_case("n") {
                            eprint!("\r\n");
                            return Ok(false);
                        }
                        // Invalid input, continue waiting
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            // Clear and reprint
                            eprint!("\r{} [Y/N]: {}", prompt, input);
                            stderr.flush()?;
                        }
                    }
                    KeyCode::Esc => {
                        eprint!("\r\n");
                        return Ok(false);
                    }
                    _ => {}
                }
            }
        }
    }
}
