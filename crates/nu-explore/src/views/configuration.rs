use std::{cmp::Ordering, fmt::Debug, ptr::addr_of};

use crossterm::event::{KeyCode, KeyEvent};
use nu_color_config::get_color_map;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use nu_table::TextStyle;
use tui::{
    layout::Rect,
    style::Style,
    widgets::{BorderType, Borders, Paragraph},
};

use crate::{
    nu_common::{truncate_str, NuText},
    pager::{Frame, Transition, ViewInfo},
    util::create_map,
    views::util::nu_style_to_tui,
};

use super::{cursor::WindowCursor, Layout, View, ViewConfig};

#[derive(Debug, Default)]
pub struct ConfigurationView {
    options: Vec<ConfigGroup>,
    peeked_cursor: Option<WindowCursor>,
    cursor: WindowCursor,
    border_color: Style,
    cursor_color: Style,
    list_color: Style,
}

impl ConfigurationView {
    pub fn new(options: Vec<ConfigGroup>) -> Self {
        let cursor = WindowCursor::new(options.len(), options.len()).expect("...");

        Self {
            options,
            cursor,
            peeked_cursor: None,
            border_color: Style::default(),
            cursor_color: Style::default(),
            list_color: Style::default(),
        }
    }

    fn update_cursors(&mut self, height: usize) {
        self.cursor.set_window(height);

        if let Some(cursor) = &mut self.peeked_cursor {
            cursor.set_window(height);
        }
    }

    fn render_option_list(
        &mut self,
        f: &mut Frame,
        area: Rect,
        list_color: Style,
        cursor_color: Style,
        layout: &mut Layout,
    ) {
        let (data, data_c) = match self.peeked_cursor {
            Some(cursor) => {
                let i = self.cursor.index();
                let opt = &self.options[i];
                let data = opt
                    .options
                    .iter()
                    .map(|e| e.name.clone())
                    .collect::<Vec<_>>();

                (data, cursor)
            }
            None => {
                let data = self
                    .options
                    .iter()
                    .map(|o| o.group.clone())
                    .collect::<Vec<_>>();

                (data, self.cursor)
            }
        };

        render_list(f, area, &data, data_c, list_color, cursor_color, layout);
    }

    fn peek_current(&self) -> Option<(&ConfigGroup, &ConfigOption)> {
        let cursor = match self.peeked_cursor {
            Some(cursor) => cursor,
            None => return None,
        };

        let i = self.cursor.index();
        let j = cursor.index();
        let group = &self.options[i];
        let opt = &group.options[j];

        Some((group, opt))
    }

    fn peek_current_group(&self) -> &ConfigGroup {
        let i = self.cursor.index();
        &self.options[i]
    }

    fn peek_current_opt(&mut self) -> Option<&mut ConfigOption> {
        let cursor = match self.peeked_cursor {
            Some(cursor) => cursor,
            None => return None,
        };

        let i = self.cursor.index();
        let j = cursor.index();

        Some(&mut self.options[i].options[j])
    }
}

#[derive(Debug, Default)]
pub struct ConfigGroup {
    group: String,
    description: String,
    options: Vec<ConfigOption>,
}

impl ConfigGroup {
    pub fn new(group: String, options: Vec<ConfigOption>, description: String) -> Self {
        Self {
            group,
            options,
            description,
        }
    }

    pub fn group(&self) -> &str {
        self.group.as_ref()
    }
}

pub struct ConfigOption {
    name: String,
    view: Box<dyn View>,
}

impl ConfigOption {
    pub fn new(name: String, view: Box<dyn View>) -> Self {
        Self { name, view }
    }
}

impl Debug for ConfigOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigOption")
            .field("name", &self.name)
            .field("view", &addr_of!(self.view))
            .finish()
    }
}

impl View for ConfigurationView {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        const LEFT_PADDING: u16 = 1;
        const BLOCK_PADDING: u16 = 1;
        const OPTION_BLOCK_WIDTH: u16 = 30;
        const USED_HEIGHT_BY_BORDERS: u16 = 2;

        if area.width < 40 {
            return;
        }

        let list_color = self.list_color;
        let border_color = self.border_color;
        let cursor_color = self.cursor_color;

        let height = area.height - USED_HEIGHT_BY_BORDERS;

        let option_b_x1 = area.x + LEFT_PADDING;
        let option_b_x2 = area.x + LEFT_PADDING + OPTION_BLOCK_WIDTH;

        let view_b_x1 = option_b_x2 + BLOCK_PADDING;
        let view_b_w = area.width - (LEFT_PADDING + BLOCK_PADDING + OPTION_BLOCK_WIDTH);

        let option_content_x1 = option_b_x1 + 1;
        let option_content_w = OPTION_BLOCK_WIDTH - 2;
        let option_content_h = height;

        let option_content_area =
            Rect::new(option_content_x1, 1, option_content_w, option_content_h);

        let view_content_x1 = view_b_x1 + 1;
        let view_content_w = view_b_w - 2;
        let view_content_h = height;

        let view_content_area = Rect::new(view_content_x1, 1, view_content_w, view_content_h);

        let option_block = tui::widgets::Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(border_color);
        let option_area = Rect::new(option_b_x1, area.y, OPTION_BLOCK_WIDTH, area.height);

        let view_block = tui::widgets::Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(border_color);
        let view_area = Rect::new(view_b_x1, area.y, view_b_w, area.height);

        f.render_widget(option_block, option_area);
        f.render_widget(view_block, view_area);

        self.render_option_list(f, option_content_area, list_color, cursor_color, layout);

        if let Some(opt) = self.peek_current_opt() {
            let mut layout = Layout::default();
            opt.view.draw(f, view_content_area, cfg, &mut layout);
        } else {
            let group = self.peek_current_group();
            let description = &group.description;

            f.render_widget(Paragraph::new(description.as_str()), view_content_area);
        }

        self.update_cursors(height as usize);
    }

    fn handle_input(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        _: &Layout,
        _: &mut ViewInfo,
        key: KeyEvent,
    ) -> Option<Transition> {
        match key.code {
            KeyCode::Esc => {
                if self.peeked_cursor.is_some() {
                    self.peeked_cursor = None;
                    Some(Transition::Ok)
                } else {
                    Some(Transition::Exit)
                }
            }
            KeyCode::Up => {
                match &mut self.peeked_cursor {
                    Some(cursor) => cursor.prev(1),
                    None => self.cursor.prev(1),
                };

                if let Some((group, opt)) = self.peek_current() {
                    return Some(Transition::Cmd(build_tweak_cmd(group, opt)));
                }

                Some(Transition::Ok)
            }
            KeyCode::Down => {
                match &mut self.peeked_cursor {
                    Some(cursor) => cursor.next(1),
                    None => self.cursor.next(1),
                };

                if let Some((group, opt)) = self.peek_current() {
                    return Some(Transition::Cmd(build_tweak_cmd(group, opt)));
                }

                Some(Transition::Ok)
            }
            KeyCode::PageUp => {
                match &mut self.peeked_cursor {
                    Some(cursor) => cursor.prev_window(),
                    None => self.cursor.prev_window(),
                };

                if let Some((group, opt)) = self.peek_current() {
                    return Some(Transition::Cmd(build_tweak_cmd(group, opt)));
                }

                Some(Transition::Ok)
            }
            KeyCode::PageDown => {
                match &mut self.peeked_cursor {
                    Some(cursor) => cursor.next_window(),
                    None => self.cursor.next_window(),
                };

                if let Some((group, opt)) = self.peek_current() {
                    return Some(Transition::Cmd(build_tweak_cmd(group, opt)));
                }

                Some(Transition::Ok)
            }
            KeyCode::Enter => {
                if self.peeked_cursor.is_some() {
                    return Some(Transition::Ok);
                }

                self.peeked_cursor = Some(WindowCursor::default());
                let length = self.peek_current().expect("...").0.options.len();

                self.peeked_cursor = WindowCursor::new(length, length);

                let (group, opt) = self.peek_current().expect("...");

                Some(Transition::Cmd(build_tweak_cmd(group, opt)))
            }
            _ => None,
        }
    }

    fn exit(&mut self) -> Option<Value> {
        None
    }

    fn collect_data(&self) -> Vec<NuText> {
        if self.peeked_cursor.is_some() {
            let i = self.cursor.index();
            let opt = &self.options[i];
            opt.options
                .iter()
                .map(|e| (e.name.clone(), TextStyle::default()))
                .collect::<Vec<_>>()
        } else {
            self.options
                .iter()
                .map(|s| (s.group.to_string(), TextStyle::default()))
                .collect()
        }
    }

    fn show_data(&mut self, i: usize) -> bool {
        if let Some(c) = &mut self.peeked_cursor {
            let i = self.cursor.index();
            if i > self.options[i].options.len() {
                return false;
            }

            loop {
                let p = c.index();
                match i.cmp(&p) {
                    Ordering::Equal => return true,
                    Ordering::Less => c.prev(1),
                    Ordering::Greater => c.next(1),
                };
            }
        } else {
            if i > self.options.len() {
                return false;
            }

            loop {
                let p = self.cursor.index();
                match i.cmp(&p) {
                    Ordering::Equal => return true,
                    Ordering::Less => self.cursor.prev(1),
                    Ordering::Greater => self.cursor.next(1),
                };
            }
        }
    }

    fn setup(&mut self, config: ViewConfig<'_>) {
        if let Some(hm) = config.config.get("config").and_then(create_map) {
            let colors = get_color_map(&hm);

            if let Some(style) = colors.get("border_color").copied() {
                self.border_color = nu_style_to_tui(style);
            }

            if let Some(style) = colors.get("cursor_color").copied() {
                self.cursor_color = nu_style_to_tui(style);
            }

            if let Some(style) = colors.get("list_color").copied() {
                self.list_color = nu_style_to_tui(style);
            }
        }

        for group in &mut self.options {
            for opt in &mut group.options {
                opt.view.setup(config);
            }
        }
    }
}

fn build_tweak_cmd(group: &ConfigGroup, opt: &ConfigOption) -> String {
    format!("tweak {} {}", group.group(), opt.name)
}

fn render_list(
    f: &mut Frame,
    area: Rect,
    data: &[String],
    cursor: WindowCursor,
    not_picked_s: Style,
    picked_s: Style,
    layout: &mut Layout,
) {
    let height = area.height as usize;
    let width = area.width as usize;

    let mut data = &data[cursor.starts_at()..];
    if data.len() > height {
        data = &data[..height];
    }

    let selected_row = cursor.offset();

    for (i, name) in data.iter().enumerate() {
        let mut name = name.to_owned();
        truncate_str(&mut name, width);

        let area = Rect::new(area.x, area.y + i as u16, area.width, 1);

        let mut text = Paragraph::new(name.clone());

        if i == selected_row {
            text = text.style(picked_s);
        } else {
            text = text.style(not_picked_s);
        }

        f.render_widget(text, area);

        layout.push(&name, area.x, area.y, area.width, 1);
    }
}
