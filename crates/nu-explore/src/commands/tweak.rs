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
            description: "Tweak different settings",
            arguments: vec![],
            examples: vec![
                HelpExample::new(":tweak table show_index false", "Don't show index anymore"),
                HelpExample::new(":tweak table show_head false", "Don't show header anymore"),
                HelpExample::new(
                    ":tweak try border_color {bg: '#FFFFFF', fg: '#F213F1'}",
                    "Make a different color for borders in :try",
                ),
            ],
            input: vec![],
        })
    }

    fn parse(&mut self, input: &str) -> Result<()> {
        let input = input.trim();

        let args = input
            .split_whitespace()
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>();

        if args.len() < 2 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "expected to get at least 2 arguments",
            ));
        }

        let path = &args[..args.len() - 1];
        let value = args[args.len() - 1];

        let value = parse_value(value);

        self.path = path.iter().map(|s| s.to_string()).collect();
        self.value = value;

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
