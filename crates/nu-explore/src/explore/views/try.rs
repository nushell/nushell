use super::super::{
    config::ExploreConfig,
    nu_common::{collect_pipeline, run_command_with_value},
    pager::{Frame, Transition, ViewInfo, report::Report},
};
use super::{Layout, Orientation, View, ViewConfig, record::RecordView, util::nu_style_to_tui};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use nu_protocol::{
    PipelineData, Value,
    engine::{EngineState, Stack},
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use std::cmp::min;
use unicode_width::UnicodeWidthStr;

pub struct TryView {
    input: Value,
    command: String,
    immediate: bool,
    table: Option<RecordView>,
    view_mode: bool,
    border_color: Style,
    config: ExploreConfig,
}

impl TryView {
    pub fn new(input: Value, config: ExploreConfig) -> Self {
        Self {
            input,
            table: None,
            immediate: config.try_reactive,
            border_color: nu_style_to_tui(config.table.separator_style),
            view_mode: false,
            command: String::new(),
            config,
        }
    }

    pub fn init(&mut self, command: String) {
        self.command = command;
    }

    pub fn try_run(&mut self, engine_state: &EngineState, stack: &mut Stack) -> Result<()> {
        let view = run_command(
            &self.command,
            &self.input,
            engine_state,
            stack,
            &self.config,
        )?;
        self.table = Some(view);
        Ok(())
    }
}

impl View for TryView {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        let border_color = self.border_color;

        // Calculate areas with better proportions
        let cmd_height: u16 = 3;
        let margin: u16 = 1;

        // Command input area at the top
        let cmd_area = Rect::new(
            area.x + margin,
            area.y,
            area.width.saturating_sub(margin * 2),
            cmd_height,
        );

        // Results area below
        let table_area = Rect::new(
            area.x + margin,
            area.y + cmd_height,
            area.width.saturating_sub(margin * 2),
            area.height.saturating_sub(cmd_height),
        );

        // Draw command input block with rounded corners
        let cmd_block = if self.view_mode {
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_color)
                .title(Line::from(vec![
                    Span::styled(" ", Style::default()),
                    Span::styled("Command", border_color),
                    Span::styled(" ", Style::default()),
                ]))
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_color.add_modifier(Modifier::BOLD))
                .title(Line::from(vec![
                    Span::styled(" ", Style::default()),
                    Span::styled("Command", border_color.add_modifier(Modifier::BOLD)),
                    Span::styled(" ▸ ", border_color.add_modifier(Modifier::BOLD)),
                ]))
        };

        f.render_widget(cmd_block, cmd_area);

        // Render the command input text
        let cmd_input_area = Rect::new(
            cmd_area.x + 2,
            cmd_area.y + 1,
            cmd_area.width.saturating_sub(4),
            1,
        );

        let input = self.command.as_str();
        let prompt = "❯ ";
        let prompt_width = prompt.width() as u16;

        let max_cmd_len = cmd_input_area.width.saturating_sub(prompt_width);
        let display_input = if input.width() as u16 > max_cmd_len {
            // Take last max_cmd_len chars when input is too long
            let take_bytes = input
                .chars()
                .rev()
                .take(max_cmd_len as usize)
                .map(|c| c.len_utf8())
                .sum::<usize>();
            let skip = input.len() - take_bytes;
            &input[skip..]
        } else {
            input
        };

        let cmd_line = Line::from(vec![
            Span::styled(prompt, border_color.add_modifier(Modifier::BOLD)),
            Span::raw(display_input),
        ]);
        let cmd_input = Paragraph::new(cmd_line);
        f.render_widget(cmd_input, cmd_input_area);

        // Position cursor at end of input when in command mode
        if !self.view_mode {
            let cursor_x =
                cmd_input_area.x + prompt_width + min(display_input.width() as u16, max_cmd_len);
            let cursor_x_max = cmd_input_area.x + cmd_input_area.width.saturating_sub(1);
            if cursor_x <= cursor_x_max {
                f.set_cursor_position((cursor_x, cmd_input_area.y));
            }
        }

        // Draw results block with rounded corners
        let table_block = if self.view_mode {
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_color.add_modifier(Modifier::BOLD))
                .title(Line::from(vec![
                    Span::styled(" ", Style::default()),
                    Span::styled("Results", border_color.add_modifier(Modifier::BOLD)),
                    Span::styled(" ◂ ", border_color.add_modifier(Modifier::BOLD)),
                ]))
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_color)
                .title(Line::from(vec![
                    Span::styled(" ", Style::default()),
                    Span::styled("Results", border_color),
                    Span::styled(" ", Style::default()),
                ]))
        };

        f.render_widget(table_block, table_area);

        // Render the table inside the results block
        if let Some(table) = &mut self.table {
            let inner_area = Rect::new(
                table_area.x + 1,
                table_area.y + 1,
                table_area.width.saturating_sub(2),
                table_area.height.saturating_sub(2),
            );

            if inner_area.width > 0 && inner_area.height > 0 {
                table.draw(f, inner_area, cfg, layout);
            }
        } else {
            // Show hint when no results yet
            let hint_area = Rect::new(
                table_area.x + 2,
                table_area.y + 1,
                table_area.width.saturating_sub(4),
                1,
            );
            let hint = Paragraph::new(Line::from(vec![
                Span::styled(
                    "Type a command and press ",
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::styled("Enter", border_color),
                Span::styled(
                    " to see results",
                    Style::default().add_modifier(Modifier::DIM),
                ),
            ]));
            f.render_widget(hint, hint_area);
        }
    }

    fn handle_input(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        layout: &Layout,
        info: &mut ViewInfo,
        key: KeyEvent,
    ) -> Transition {
        if self.view_mode {
            let table = self
                .table
                .as_mut()
                .expect("we know that we have a table cause of a flag");

            let was_at_the_top = table.get_cursor_position().row == 0;

            if was_at_the_top && matches!(key.code, KeyCode::Up | KeyCode::PageUp) {
                self.view_mode = false;
                return Transition::Ok;
            }

            if let KeyCode::Tab = key.code {
                self.view_mode = false;
                return Transition::Ok;
            }

            let result = table.handle_input(engine_state, stack, layout, info, key);

            return match result {
                Transition::Ok | Transition::Cmd { .. } => Transition::Ok,
                Transition::Exit => {
                    self.view_mode = false;
                    Transition::Ok
                }
                Transition::None => Transition::None,
            };
        }

        match &key.code {
            KeyCode::Esc => Transition::Exit,
            KeyCode::Backspace => {
                if !self.command.is_empty() {
                    self.command.pop();

                    if self.immediate {
                        match self.try_run(engine_state, stack) {
                            Ok(_) => info.report = Some(Report::default()),
                            Err(err) => info.report = Some(Report::error(format!("Error: {err}"))),
                        }
                    }
                }

                Transition::Ok
            }
            KeyCode::Char(c) => {
                self.command.push(*c);

                if self.immediate {
                    match self.try_run(engine_state, stack) {
                        Ok(_) => info.report = Some(Report::default()),
                        Err(err) => info.report = Some(Report::error(format!("Error: {err}"))),
                    }
                }

                Transition::Ok
            }
            KeyCode::Down | KeyCode::Tab => {
                if self.table.is_some() {
                    self.view_mode = true;
                }

                Transition::Ok
            }
            KeyCode::Enter => {
                match self.try_run(engine_state, stack) {
                    Ok(_) => info.report = Some(Report::default()),
                    Err(err) => info.report = Some(Report::error(format!("Error: {err}"))),
                }

                Transition::Ok
            }
            _ => Transition::None,
        }
    }

    fn exit(&mut self) -> Option<Value> {
        self.table.as_mut().and_then(|v| v.exit())
    }

    fn collect_data(&self) -> Vec<super::super::nu_common::NuText> {
        self.table
            .as_ref()
            .map_or_else(Vec::new, |v| v.collect_data())
    }

    fn show_data(&mut self, i: usize) -> bool {
        self.table.as_mut().is_some_and(|v| v.show_data(i))
    }
}

fn run_command(
    command: &str,
    input: &Value,
    engine_state: &EngineState,
    stack: &mut Stack,
    config: &ExploreConfig,
) -> Result<RecordView> {
    let pipeline = run_command_with_value(command, input, engine_state, stack)?;

    let is_record = matches!(pipeline, PipelineData::Value(Value::Record { .. }, ..));

    let (columns, values) = collect_pipeline(pipeline)?;

    let mut view = RecordView::new(columns, values, config.clone());
    if is_record {
        view.set_top_layer_orientation(Orientation::Left);
    }

    Ok(view)
}
