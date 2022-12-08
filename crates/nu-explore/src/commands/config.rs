use std::io::Result;

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::{
    command::Command,
    nu_common::{nu_str, NuSpan},
    views::{configuration, ConfigurationView, Preview},
};

use super::{default_color_list, ConfigOption, HelpManual, ViewCommand};

#[derive(Default, Clone)]
pub struct ConfigCmd {
    commands: Vec<Command>,
    groups: Vec<ConfigOption>,
}

impl ConfigCmd {
    pub const NAME: &'static str = "config";

    pub fn from_commands(commands: Vec<Command>) -> Self {
        Self {
            commands,
            groups: Vec::new(),
        }
    }

    pub fn register_group(&mut self, group: ConfigOption) {
        self.groups.push(group);
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
        Some(HelpManual {
            name: "config",
            description: "",
            arguments: vec![],
            examples: vec![],
            config_options: vec![
                super::ConfigOption::new(
                    ".... 1",
                    ".",
                    "config.border_color",
                    default_color_list(),
                ),
                super::ConfigOption::new(".... 2", ".", "config.list_color", default_color_list()),
                super::ConfigOption::new(
                    ".... 3",
                    ".",
                    "config.cursor_color",
                    default_color_list(),
                ),
            ],
            input: vec![],
        })
    }

    fn display_config_option(&mut self, _: String, _: String, _: String) -> bool {
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
        let mut options = vec![];

        let default_table = create_default_value();
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

                    let can_be_displayed = cmd.display_config_option(
                        opt.group.clone(),
                        opt.key.clone(),
                        value.example.to_string(),
                    );
                    let view = if can_be_displayed {
                        cmd.spawn(engine_state, stack, Some(default_table.clone()))?
                    } else {
                        Box::new(Preview::new(&opt.description))
                    };

                    let option = configuration::ConfigOption::new(value.example.to_string(), view);
                    values.push(option);
                }

                let group = configuration::ConfigGroup::new(opt.key, values);
                options.push(group);
            }
        }

        for group in &self.groups {
            let mut values = vec![];
            for value in &group.values {
                let view = Box::new(Preview::new(&group.description));

                let option = configuration::ConfigOption::new(value.example.to_string(), view);
                values.push(option);
            }

            let group = configuration::ConfigGroup::new(group.key.clone(), values);
            options.push(group);
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
