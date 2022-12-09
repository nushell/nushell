mod coloredtextw;
mod cursorw;
mod information;
mod interative;
mod preview;
mod record;
pub mod util;

use crossterm::event::KeyEvent;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use tui::layout::Rect;

use crate::{
    nu_common::{NuConfig, NuStyleTable},
    pager::ConfigMap,
};

use super::{
    nu_common::NuText,
    pager::{Frame, Transition, ViewInfo},
};

pub mod configuration;

pub use configuration::ConfigurationView;
pub use information::InformationView;
pub use interative::InteractiveView;
pub use preview::Preview;
pub use record::{Orientation, RecordView, RecordViewState};

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
    pub color_hm: &'a NuStyleTable,
    pub config: &'a ConfigMap,
}

impl<'a> ViewConfig<'a> {
    pub fn new(nu_config: &'a NuConfig, color_hm: &'a NuStyleTable, config: &'a ConfigMap) -> Self {
        Self {
            nu_config,
            color_hm,
            config,
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
    ) -> Option<Transition>;

    fn show_data(&mut self, _: usize) -> bool {
        false
    }

    fn collect_data(&self) -> Vec<NuText> {
        Vec::new()
    }

    fn exit(&mut self) -> Option<Value> {
        None
    }

    fn setup(&mut self, _: ViewConfig<'_>) {}
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
    ) -> Option<Transition> {
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

    fn setup(&mut self, cfg: ViewConfig<'_>) {
        self.as_mut().setup(cfg)
    }
}
