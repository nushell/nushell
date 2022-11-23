use crossterm::event::KeyEvent;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use tui::layout::Rect;

use super::pager::{Frame, NuText, Transition, ViewConfig, ViewInfo};

mod information;
mod interative;
mod record;

pub use information::InformationView;
pub use interative::InteractiveView;
pub use record::{RecordView, RecordViewState};

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

pub trait View {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: &ViewConfig, layout: &mut Layout);

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
}

impl View for Box<dyn View> {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: &ViewConfig, layout: &mut Layout) {
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
}
