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

    fn get_config_settings(&self) -> Vec<super::ConfigOption> {
        vec![]
    }

    fn set_config_settings(&mut self, group: String, key: String, value: String) {}

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

            let cmd_options = cmd.get_config_settings();

            for opt in cmd_options {
                let mut values = vec![];
                for value in opt.values {
                    let mut cmd = cmd.clone();
                    cmd.set_config_settings(
                        opt.group.clone(),
                        opt.key.clone(),
                        value.example.to_owned(),
                    );
                    let view = cmd.spawn(engine_state, stack, Some(default_table.clone()))?;

                    let option = ConfigOption::new(value.example.to_owned(), view);
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
