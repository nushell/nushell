//! The explore regex command implementation.

// Borrowed from the ut project and tweaked. Thanks!
// https://github.com/ksdme/ut
// Below is the ut license:
// MIT License
//
// Copyright (c) 2025 Kilari Teja
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::explore_regex::app::App;
use crate::explore_regex::ui::run_app_loop;
use nu_engine::command_prelude::*;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};
use std::io;

/// A `regular expression explorer` program.
#[derive(Clone)]
pub struct ExploreRegex;

impl Command for ExploreRegex {
    fn name(&self) -> &str {
        "explore regex"
    }

    fn description(&self) -> &str {
        "Launch a TUI to create and explore regular expressions interactively."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("explore regex")
            .input_output_types(vec![
                (Type::Nothing, Type::String),
                (Type::String, Type::String),
            ])
            .category(Category::Viewers)
    }

    fn extra_description(&self) -> &str {
        r#"Press `Ctrl-Q` to quit and provide constructed regular expression as the output."#
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let input_span = input.span().unwrap_or(call.head);
        let (string_input, _span, _metadata) = input.collect_string_strict(input_span)?;
        let regex = execute_regex_app(call, string_input)?;

        Ok(PipelineData::Value(
            nu_protocol::Value::string(regex, call.head),
            None,
        ))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Explore a regular expression interactively",
                example: r#"explore regex"#,
                result: None,
            },
            Example {
                description: "Explore a regular expression interactively with sample text",
                example: r#"open -r Cargo.toml | explore regex"#,
                result: None,
            },
        ]
    }
}

fn execute_regex_app(call: &Call, string_input: String) -> Result<String, ShellError> {
    // Setup terminal
    enable_raw_mode().map_err(|e| ShellError::GenericError {
        error: "Could not enable raw mode".into(),
        msg: format!("terminal error: {e}"),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
        ShellError::GenericError {
            error: "Could not enter alternate screen".into(),
            msg: format!("terminal error: {e}"),
            span: Some(call.head),
            help: None,
            inner: vec![],
        }
    })?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| ShellError::GenericError {
        error: "Could not initialize terminal".into(),
        msg: format!("terminal error: {e}"),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let mut app = App::new(string_input);
    let res = run_app_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().map_err(|e| ShellError::GenericError {
        error: "Could not disable raw mode".into(),
        msg: format!("terminal error: {e}"),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| ShellError::GenericError {
        error: "Could not leave alternate screen".into(),
        msg: format!("terminal error: {e}"),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    terminal
        .show_cursor()
        .map_err(|e| ShellError::GenericError {
            error: "Could not show terminal cursor".into(),
            msg: format!("terminal error: {e}"),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    res.map_err(|err| ShellError::GenericError {
        error: "Application error".into(),
        msg: format!("application error: {err}"),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    Ok(app.get_regex_input())
}
