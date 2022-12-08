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
    nu_common::{collect_pipeline, has_simple_value, nu_str, run_command_with_value, NuSpan},
    pager::Frame,
    views::{
        configuration::{ConfigGroup, ConfigOption},
        ConfigurationView, Layout, Orientation, Preview, RecordView, View, ViewConfig,
    },
};

use super::{HelpExample, HelpManual, ViewCommand};

#[derive(Default, Clone)]
pub struct ConfigCmd {
    commands: Vec<Command>,
}

impl ConfigCmd {
    pub const NAME: &'static str = "config";

    pub fn new(commands: Vec<Command>) -> Self {
        Self { commands }
    }
}

impl ViewCommand for ConfigCmd {
    type View = ConfigurationView;

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

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn spawn(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        _: Option<Value>,
    ) -> Result<Self::View> {
        let default_table = create_default_value();

        let mut options = vec![];

        for cmd in &self.commands {
            let cmd = match cmd {
                Command::Reactive(_) => continue,
                Command::View { cmd, .. } => cmd,
            };

            let help = match cmd.help() {
                Some(help) => help,
                None => continue,
            };

            for opt in help.config_options {
                let mut values = vec![];
                for value in opt.values {
                    let mut cmd = cmd.clone();

                    let can_be_displayed =
                        cmd.display_config_option(opt.group.clone(), opt.key.clone(), value.example.to_string());
                    let view = if can_be_displayed {
                        cmd.spawn(engine_state, stack, Some(default_table.clone()))?
                    } else {
                        Box::new(Preview::new(&opt.description))
                    };

                    let option = ConfigOption::new(value.example.to_string(), view);
                    values.push(option);
                }

                let group = ConfigGroup::new(opt.key, values);
                options.push(group);
            }
        }

        options.sort_by(|x, y| x.group().cmp(y.group()));

        Ok(ConfigurationView::new(options))
    }
}

fn create_default_value() -> Value {
    let span = NuSpan::unknown();

    let record = |i: usize| Value::Record {
        cols: vec![String::from("key"), String::from("value")],
        vals: vec![nu_str(format!("key-{}", i)), nu_str(format!("{}", i))],
        span,
    };

    Value::List {
        vals: vec![record(0), record(1), record(2)],
        span,
    }
}
