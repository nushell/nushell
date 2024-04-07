use super::{
    record::{RecordView, TableTheme},
    util::{lookup_tui_color, nu_style_to_tui},
    Layout, Orientation, View, ViewConfig,
};
use crate::{
    nu_common::{collect_pipeline, run_command_with_value},
    pager::{report::Report, Frame, Transition, ViewInfo},
    util::create_map,
};
use crossterm::event::{KeyCode, KeyEvent};
use nu_color_config::get_color_map;
use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{BorderType, Borders, Paragraph},
};
use std::cmp::min;

pub struct InteractiveView<'a> {
    input: Value,
    command: String,
    immediate: bool,
    table: Option<RecordView<'a>>,
    table_theme: TableTheme,
    view_mode: bool,
    border_color: Style,
    highlighted_color: Style,
}

impl<'a> InteractiveView<'a> {
    pub fn new(input: Value) -> Self {
        Self {
            input,
            table: None,
            immediate: false,
            table_theme: TableTheme::default(),
            border_color: Style::default(),
            highlighted_color: Style::default(),
            view_mode: false,
            command: String::new(),
        }
    }

    pub fn init(&mut self, command: String) {
        self.command = command;
    }

    pub fn try_run(&mut self, engine_state: &EngineState, stack: &mut Stack) -> Result<(), String> {
        let mut view = run_command(&self.command, &self.input, engine_state, stack)?;
        view.set_theme(self.table_theme.clone());

        self.table = Some(view);
        Ok(())
    }
}

impl View for InteractiveView<'_> {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        let border_color = self.border_color;
        let highlighted_color = self.highlighted_color;

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
                .border_style(highlighted_color)
        };

        f.render_widget(cmd_block, cmd_area);

        let cmd_input_area = Rect::new(
            cmd_area.x + 2,
            cmd_area.y + 1,
            cmd_area.width - 2 - 2 - 1,
            1,
        );

        let mut input = self.command.as_str();

        let max_cmd_len = min(input.len() as u16, cmd_input_area.width);
        if input.len() as u16 > max_cmd_len {
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
                f.set_cursor(area.x + 1 + 1 + 1 + max_cmd_len, area.y + 1);
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
                .border_style(highlighted_color)
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
    ) -> Option<Transition> {
        if self.view_mode {
            let table = self
                .table
                .as_mut()
                .expect("we know that we have a table cause of a flag");

            let was_at_the_top = table.get_current_position().0 == 0;

            if was_at_the_top && matches!(key.code, KeyCode::Up | KeyCode::PageUp) {
                self.view_mode = false;
                return Some(Transition::Ok);
            }

            if matches!(key.code, KeyCode::Tab) {
                self.view_mode = false;
                return Some(Transition::Ok);
            }

            let result = table.handle_input(engine_state, stack, layout, info, key);

            return match result {
                Some(Transition::Ok | Transition::Cmd { .. }) => Some(Transition::Ok),
                Some(Transition::Exit) => {
                    self.view_mode = false;
                    Some(Transition::Ok)
                }
                None => None,
            };
        }

        match &key.code {
            KeyCode::Esc => Some(Transition::Exit),
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

                Some(Transition::Ok)
            }
            KeyCode::Char(c) => {
                self.command.push(*c);

                if self.immediate {
                    match self.try_run(engine_state, stack) {
                        Ok(_) => info.report = Some(Report::default()),
                        Err(err) => info.report = Some(Report::error(format!("Error: {err}"))),
                    }
                }

                Some(Transition::Ok)
            }
            KeyCode::Down | KeyCode::Tab => {
                if self.table.is_some() {
                    self.view_mode = true;
                }

                Some(Transition::Ok)
            }
            KeyCode::Enter => {
                match self.try_run(engine_state, stack) {
                    Ok(_) => info.report = Some(Report::default()),
                    Err(err) => info.report = Some(Report::error(format!("Error: {err}"))),
                }

                Some(Transition::Ok)
            }
            _ => None,
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
        self.table.as_mut().map_or(false, |v| v.show_data(i))
    }

    fn setup(&mut self, config: ViewConfig<'_>) {
        self.border_color = lookup_tui_color(config.style_computer, "separator");

        if let Some(hm) = config.config.get("try").and_then(create_map) {
            let colors = get_color_map(&hm);

            if let Some(color) = colors.get("highlighted_color").copied() {
                self.highlighted_color = nu_style_to_tui(color);
            }

            if self.border_color != Style::default() && self.highlighted_color == Style::default() {
                self.highlighted_color = self.border_color;
            }

            if let Some(val) = hm.get("reactive").and_then(|v| v.as_bool().ok()) {
                self.immediate = val;
            }
        }

        let mut r = RecordView::new(vec![], vec![]);
        r.setup(config);

        self.table_theme = r.get_theme().clone();

        if let Some(view) = &mut self.table {
            view.set_theme(self.table_theme.clone());
            view.set_orientation(r.get_orientation_current());
            view.set_orientation_current(r.get_orientation_current());
        }
    }
}

fn run_command(
    command: &str,
    input: &Value,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<RecordView<'static>, String> {
    let pipeline =
        run_command_with_value(command, input, engine_state, stack).map_err(|e| e.to_string())?;

    let is_record = matches!(pipeline, PipelineData::Value(Value::Record { .. }, ..));

    let (columns, values) = collect_pipeline(pipeline);

    let mut view = RecordView::new(columns, values);
    if is_record {
        view.set_orientation_current(Orientation::Left);
    }

    Ok(view)
}
