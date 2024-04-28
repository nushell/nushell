use super::ViewCommand;
use crate::views::Preview;
use anyhow::Result;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

#[derive(Debug, Default, Clone)]
pub struct HelpCmd {}

impl HelpCmd {
    pub const NAME: &'static str = "help";
    pub fn view() -> Preview {
        Preview::new(HELP_MESSAGE)
    }
}

// TODO: search help could use some updating... search results get shown immediately after typing, don't need to press Enter
const HELP_MESSAGE: &str = r#"# Explore

Explore helps you dynamically navigate through your data

## Basics

                           Move around:  Use the cursor keys
        Drill down into records+tables:  Press <Enter> to select a cell, move around with cursor keys, then press <Enter> again
                    Go back/up a level:  Press <Esc>
Transpose data (flip rows and columns):  Press "t"
    Expand data (show all nested data):  Press "e"
                  Open this help page :  Type ":help" then <Enter>
              Open an interactive REPL:  Type ":try" then <Enter>
                        Scroll up/down:  Use the "Page Up" and "Page Down" keys
                          Exit Explore:  Type ":q" then <Enter>, or Ctrl+D. Alternately, press <Esc> until Explore exits

## Search

Most commands support search via regular expressions.

You can type "/" and type a pattern you want to search on.
Then hit <Enter> and you will see the search results.

To go to the next hit use "<n>" key.

You also can do a reverse search by using "?" instead of "/".
"#;

impl ViewCommand for HelpCmd {
    type View = Preview;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn spawn(&mut self, _: &EngineState, _: &mut Stack, _: Option<Value>) -> Result<Self::View> {
        Ok(HelpCmd::view())
    }
}
