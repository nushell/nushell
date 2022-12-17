use std::io::{self, Result};

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::{
    nu_common::NuSpan,
    pager::{Pager, Transition},
};

use super::{HelpExample, HelpManual, SimpleCommand};

#[derive(Default, Clone)]
pub struct TweakCmd {
    path: Vec<String>,
    value: Value,
}

impl TweakCmd {
    pub const NAME: &'static str = "tweak";
}

impl SimpleCommand for TweakCmd {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        Some(HelpManual {
            name: "tweak",
            description: "Set `explore` settings.\nLike a non-interactive version of :config",
            arguments: vec![],
            examples: vec![
                HelpExample::new(":tweak table.show_index false", "Don't show index anymore"),
                HelpExample::new(":tweak table.show_head false", "Don't show header anymore"),
                HelpExample::new(
                    ":tweak try.border_color {bg: '#FFFFFF', fg: '#F213F1'}",
                    "Make a different color for borders in :try",
                ),
            ],
            config_options: vec![],
            input: vec![],
        })
    }

    fn parse(&mut self, input: &str) -> Result<()> {
        let input = input.trim();

        let args = input.split_once(' ');
        let (key, value) = match args {
            Some(args) => args,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "expected to get 2 arguments 'key value'",
                ))
            }
        };

        self.value = parse_value(value);

        self.path = key
            .split_terminator('.')
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        Ok(())
    }

    fn react(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        p: &mut Pager<'_>,
        _: Option<Value>,
    ) -> Result<Transition> {
        p.set_config(&self.path, self.value.clone());

        Ok(Transition::Ok)
    }
}

fn parse_value(value: &str) -> Value {
    match value {
        "true" => Value::boolean(true, NuSpan::unknown()),
        "false" => Value::boolean(false, NuSpan::unknown()),
        s => Value::string(s.to_owned(), NuSpan::unknown()),
    }
}
