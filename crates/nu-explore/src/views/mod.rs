mod binary;
mod colored_text_widget;
mod cursor;
mod preview;
mod record;
mod r#try;
pub mod util;

use super::{
    nu_common::NuText,
    pager::{Frame, Transition, ViewInfo},
};
use crate::{explore::ExploreConfig, nu_common::NuConfig};
use crossterm::event::KeyEvent;
use lscolors::LsColors;
use nu_color_config::StyleComputer;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use ratatui::layout::Rect;

pub use binary::BinaryView;
pub use preview::Preview;
pub use r#try::TryView;
pub use record::{Orientation, RecordView};

#[derive(Debug, Default)]
pub struct Layout {
    pub data: Vec<ElementInfo>,
}

impl Layout {
    fn push(&mut self, text: &str, x: u16, y: u16, width: u16, height: u16) {
        self.data.push(ElementInfo::new(text, x, y, width, height));
    }
}

#[derive(Debug, Default, Clone)]
pub struct ElementInfo {
    // todo: make it a Cow
    pub text: String,
    pub area: Rect,
}

impl ElementInfo {
    pub fn new(text: impl Into<String>, x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            text: text.into(),
            area: Rect::new(x, y, width, height),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ViewConfig<'a> {
    pub nu_config: &'a NuConfig,
    pub explore_config: &'a ExploreConfig,
    pub style_computer: &'a StyleComputer<'a>,
    pub lscolors: &'a LsColors,
    pub cwd: &'a str,
}

impl<'a> ViewConfig<'a> {
    pub fn new(
        nu_config: &'a NuConfig,
        explore_config: &'a ExploreConfig,
        style_computer: &'a StyleComputer<'a>,
        lscolors: &'a LsColors,
        cwd: &'a str,
    ) -> Self {
        Self {
            nu_config,
            explore_config,
            style_computer,
            lscolors,
            cwd,
        }
    }
}

pub trait View {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout);

    fn handle_input(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        layout: &Layout,
        info: &mut ViewInfo,
        key: KeyEvent,
    ) -> Transition;

    fn show_data(&mut self, _: usize) -> bool {
        false
    }

    fn collect_data(&self) -> Vec<NuText> {
        Vec::new()
    }

    fn exit(&mut self) -> Option<Value> {
        None
    }
}

impl View for Box<dyn View> {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        self.as_mut().draw(f, area, cfg, layout)
    }

    fn handle_input(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        layout: &Layout,
        info: &mut ViewInfo,
        key: KeyEvent,
    ) -> Transition {
        self.as_mut()
            .handle_input(engine_state, stack, layout, info, key)
    }

    fn collect_data(&self) -> Vec<NuText> {
        self.as_ref().collect_data()
    }

    fn exit(&mut self) -> Option<Value> {
        self.as_mut().exit()
    }

    fn show_data(&mut self, i: usize) -> bool {
        self.as_mut().show_data(i)
    }
}
