use super::{Layout, Orientation, View, ViewConfig, record::RecordView, util::nu_style_to_tui};
use crate::{
    explore::ExploreConfig,
    nu_common::{collect_pipeline, run_command_with_value},
    pager::{Frame, Transition, ViewInfo, report::Report},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use nu_protocol::{
    PipelineData, Value,
    engine::{EngineState, Stack},
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{BorderType, Borders, Paragraph},
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

        let cmd_block = ratatui::widgets::Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(border_color);
        let cmd_area = Rect::new(area.x + 1, area.y, area.width - 2, 3);

        let cmd_block = if self.view_mode {
            cmd_block
        } else {
            cmd_block
                .border_style(Style::default().add_modifier(Modifier::BOLD))
                .border_type(BorderType::Double)
                .border_style(border_color)
        };

        f.render_widget(cmd_block, cmd_area);

        let cmd_input_area = Rect::new(
            cmd_area.x + 2,
            cmd_area.y + 1,
            cmd_area.width - 2 - 2 - 1,
            1,
        );

        let mut input = self.command.as_str();

        let max_cmd_len = min(input.width() as u16, cmd_input_area.width);
        if input.width() as u16 > max_cmd_len {
            // in such case we take last max_cmd_len chars
            let take_bytes = input
                .chars()
                .rev()
                .take(max_cmd_len as usize)
                .map(|c| c.len_utf8())
                .sum::<usize>();
            let skip = input.len() - take_bytes;

            input = &input[skip..];
        }

        let cmd_input = Paragraph::new(input);

        f.render_widget(cmd_input, cmd_input_area);

        if !self.view_mode {
            let cur_w = area.x + 1 + 1 + 1 + max_cmd_len;
            let cur_w_max = area.x + 1 + 1 + 1 + area.width - 2 - 1 - 1 - 1 - 1;
            if cur_w < cur_w_max {
                f.set_cursor_position((area.x + 1 + 1 + 1 + max_cmd_len, area.y + 1));
            }
        }

        let table_block = ratatui::widgets::Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(border_color);
        let table_area = Rect::new(area.x + 1, area.y + 3, area.width - 2, area.height - 3);

        let table_block = if self.view_mode {
            table_block
                .border_style(Style::default().add_modifier(Modifier::BOLD))
                .border_type(BorderType::Double)
                .border_style(border_color)
        } else {
            table_block
        };

        f.render_widget(table_block, table_area);

        if let Some(table) = &mut self.table {
            let area = Rect::new(
                area.x + 2,
                area.y + 4,
                area.width - 3 - 1,
                area.height - 3 - 1 - 1,
            );

            table.draw(f, area, cfg, layout);
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

    fn collect_data(&self) -> Vec<crate::nu_common::NuText> {
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
