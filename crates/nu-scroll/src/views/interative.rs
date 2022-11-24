use std::cmp::min;

use crossterm::event::KeyEvent;
use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use reedline::KeyCode;
use tui::{
    layout::Rect,
    widgets::{BorderType, Borders, Paragraph},
};

use crate::{
    nu_common::{collect_pipeline, run_nu_command},
    pager::{Frame, Report, TableConfig, Transition, ViewConfig, ViewInfo},
};

use super::{record::RecordView, Layout, View};

pub struct InteractiveView<'a> {
    input: Value,
    command: String,
    table: Option<RecordView<'a>>,
    view_mode: bool,
    // todo: impl Debug for it
    table_cfg: TableConfig,
}

impl<'a> InteractiveView<'a> {
    pub fn new(input: Value, table_cfg: TableConfig) -> Self {
        Self {
            input,
            table_cfg,
            table: None,
            view_mode: false,
            command: String::new(),
        }
    }

    pub fn init(&mut self, command: String) {
        self.command = command;
    }
}

impl View for InteractiveView<'_> {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: &ViewConfig, layout: &mut Layout) {
        let cmd_block = tui::widgets::Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        let cmd_area = Rect::new(area.x + 1, area.y, area.width - 2, 3);

        f.render_widget(cmd_block, cmd_area);

        let cmd = Paragraph::new(self.command.clone());
        let cmd_area = Rect::new(
            area.x + 1 + 1 + 1,
            area.y + 1,
            area.width - 2 - 1 - 1 - 1 - 1 - 1,
            1,
        );
        f.render_widget(cmd, cmd_area);

        let max_cmd_len = min(
            self.command.len() as u16,
            area.width - 2 - 1 - 1 - 1 - 1 - 1,
        );
        let cur_w = area.x + 1 + 1 + 1 + max_cmd_len as u16;
        let cur_w_max = area.x + 1 + 1 + 1 + area.width - 2 - 1 - 1 - 1 - 1;
        if cur_w < cur_w_max {
            f.set_cursor(area.x + 1 + 1 + 1 + max_cmd_len as u16, area.y + 1);
        }

        let table_block = tui::widgets::Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        let table_area = Rect::new(area.x + 1, area.y + 3, area.width - 2, area.height - 3);
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
            let table = self.table.as_mut().unwrap();
            let result = table.handle_input(engine_state, stack, layout, info, key);

            match result {
                Some(Transition::Ok | Transition::Cmd(..)) => return Some(Transition::Ok),
                Some(Transition::Exit) => {
                    self.view_mode = false;
                    return Some(Transition::Ok);
                }
                None => return None,
            }
        }

        match &key.code {
            KeyCode::Esc => Some(Transition::Exit),
            KeyCode::Backspace => {
                if !self.command.is_empty() {
                    self.command.pop();
                }

                Some(Transition::Ok)
            }
            KeyCode::Char(c) => {
                self.command.push(*c);
                Some(Transition::Ok)
            }
            KeyCode::Down => {
                if self.table.is_some() {
                    self.view_mode = true;
                }

                Some(Transition::Ok)
            }
            KeyCode::Enter => {
                let pipeline = PipelineData::Value(self.input.clone(), None);
                let pipeline = run_nu_command(engine_state, stack, &self.command, pipeline);

                match pipeline {
                    Ok(pipeline_data) => {
                        let (columns, values) = collect_pipeline(pipeline_data);
                        let view = RecordView::new(columns, values, self.table_cfg.clone());

                        self.table = Some(view);
                    }
                    Err(err) => {
                        info.report = Some(Report::error(format!("Error: {}", err)));
                    }
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
}
