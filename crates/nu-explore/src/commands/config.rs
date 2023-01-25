use std::io::Result;

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::{
    nu_common::{nu_str, NuSpan},
    registry::Command,
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
        let config_options = vec![
            ConfigOption::new(
                ":config options",
                "A border color of menus",
                "config.border_color",
                default_color_list(),
            ),
            ConfigOption::new(
                ":config options",
                "Set a color of entries in a list",
                "config.list_color",
                default_color_list(),
            ),
            ConfigOption::new(
                ":config options",
                "Set a color of a chosen entry in a list",
                "config.cursor_color",
                default_color_list(),
            ),
        ];

        Some(HelpManual {
            name: Self::NAME,
            description:
                "Interactive configuration manager.\nCan be used to set various explore settings.\n\nLike an interactive version of :tweak",
            config_options,
            arguments: vec![],
            examples: vec![],
            input: vec![],
        })
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn display_config_option(&mut self, _: String, _: String, _: String) -> bool {
        false
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

                let group = configuration::ConfigGroup::new(opt.key, values, opt.description);
                options.push((opt.group, group));
            }
        }

        for opt in &self.groups {
            let mut values = vec![];
            for value in &opt.values {
                let view = Box::new(Preview::new(&opt.description));

                let option = configuration::ConfigOption::new(value.example.to_string(), view);
                values.push(option);
            }

            let group =
                configuration::ConfigGroup::new(opt.key.clone(), values, opt.description.clone());
            options.push((opt.group.clone(), group));
        }

        options.sort_by(|(group1, opt1), (group2, opt2)| {
            group1.cmp(group2).then(opt1.group().cmp(opt2.group()))
        });

        let options = options.into_iter().map(|(_, opt)| opt).collect();

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
