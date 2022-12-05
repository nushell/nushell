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
pub struct TweekCmd {
    key: String,
    value: Value,
}

impl TweekCmd {
    pub const NAME: &'static str = "tweek";
}

impl SimpleCommand for TweekCmd {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        Some(HelpManual {
            name: "tweek",
            description: "Tweek different settings",
            arguments: vec![],
            examples: vec![HelpExample::new(
                ":tweek table_show_index false",
                "Don't show index anymore",
            )],
            input: vec![],
        })
    }

    fn parse(&mut self, input: &str) -> Result<()> {
        let input = input.trim();

        let args = input.split_whitespace().collect::<Vec<_>>();
        if args.len() < 2 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "expected to get 2 arguments",
            ));
        }

        let key = args[0].to_owned();
        let value = args[1];

        let value = match value {
            "true" => Value::boolean(true, NuSpan::unknown()),
            "false" => Value::boolean(false, NuSpan::unknown()),
            s => Value::string(s.to_owned(), NuSpan::unknown()),
        };

        self.key = key;
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
        p.set_config(self.key.clone(), self.value.clone());

        Ok(Transition::Ok)
    }
}
