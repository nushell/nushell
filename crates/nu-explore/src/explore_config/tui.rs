//! TUI runtime functions for running the explore config application.

use crate::explore_config::input::{
    handle_editor_editing_input, handle_editor_normal_input, handle_tree_input,
};
use crate::explore_config::types::{App, AppResult, EditorMode, Focus, NuValueType};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::io;

/// Run the TUI and return the modified JSON data if changes were made in config mode
pub fn run_config_tui(
    json_data: Value,
    output_file: Option<String>,
    config_mode: bool,
    nu_type_map: Option<HashMap<String, NuValueType>>,
    doc_map: Option<HashMap<String, String>>,
) -> Result<Option<Value>, Box<dyn Error>> {
    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear the screen initially
    terminal.clear()?;

    let mut app = App::new(json_data, output_file, config_mode, nu_type_map, doc_map);

    // Select the first item
    app.tree_state.select_first();
    app.force_update_editor();

    let res = run_config_app(&mut terminal, &mut app);

    // Restore terminal - this is critical for clean exit
    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
        return Ok(None);
    }

    // Return the modified data if in config mode and changes were made
    if config_mode && app.modified {
        Ok(Some(app.json_data))
    } else {
        Ok(None)
    }
}

fn run_config_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Global keybindings
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Err(e) = app.save_to_file() {
                        app.status_message = format!("✗ Save failed: {}", e);
                    }
                    continue;
                }
                _ => {}
            }

            // Handle based on focus and mode
            let result = match (app.focus, app.editor_mode) {
                (Focus::Tree, _) => handle_tree_input(app, key.code, key.modifiers),
                (Focus::Editor, EditorMode::Normal) => {
                    handle_editor_normal_input(app, key.code, key.modifiers)
                }
                (Focus::Editor, EditorMode::Editing) => {
                    handle_editor_editing_input(app, key.code, key.modifiers)
                }
            };

            if let AppResult::Quit = result {
                return Ok(());
            }
        }
    }
}
