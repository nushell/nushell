use std::{
    collections::HashMap,
    io::{self, Result},
};

use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use tui::layout::Rect;

use crate::{
    command::Command,
    nu_common::{
        collect_pipeline, has_simple_value, nu_str, run_command_with_value, try_build_table, NuSpan,
    },
    pager::Frame,
    util::map_into_value,
    views::{
        configuration::{ConfigGroup, ConfigOption},
        ConfigurationView, Layout, Orientation, Preview, RecordView, View, ViewConfig,
    },
};

use super::{HelpExample, HelpManual, ViewCommand};

#[derive(Clone)]
pub struct ConfigShowCmd {
    format: ConfigFormat,
}

#[derive(Clone)]
enum ConfigFormat {
    Table,
    Nu,
}

impl ConfigShowCmd {
    pub fn new() -> Self {
        ConfigShowCmd {
            format: ConfigFormat::Table,
        }
    }
}

impl ConfigShowCmd {
    pub const NAME: &'static str = "config-show";
}

impl ViewCommand for ConfigShowCmd {
    type View = ConfigView;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        None
    }

    fn display_config_option(&mut self, group: String, key: String, value: String) -> bool {
        false
    }

    fn parse(&mut self, args: &str) -> Result<()> {
        if args.trim() == "nu" {
            self.format = ConfigFormat::Nu;
        }

        Ok(())
    }

    fn spawn(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        _: Option<Value>,
    ) -> Result<Self::View> {
        Ok(ConfigView {
            preview: Preview::new(""),
            format: self.format.clone(),
        })
    }
}

pub struct ConfigView {
    preview: Preview,
    format: ConfigFormat,
}

impl View for ConfigView {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        self.preview.draw(f, area, cfg, layout)
    }

    fn handle_input(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        layout: &Layout,
        info: &mut crate::pager::ViewInfo,
        key: crossterm::event::KeyEvent,
    ) -> Option<crate::pager::Transition> {
        self.preview
            .handle_input(engine_state, stack, layout, info, key)
    }

    fn setup(&mut self, config: ViewConfig<'_>) {
        let text = match self.format {
            ConfigFormat::Table => {
                let value = map_into_value(config.config.clone());
                try_build_table(None, config.nu_config, config.color_hm, value)
            }
            ConfigFormat::Nu => nu_json::to_string(&config.config).unwrap_or_default(),
        };

        self.preview = Preview::new(&text);
    }
}
