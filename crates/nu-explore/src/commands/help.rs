use super::ViewCommand;
use crate::views::{Preview, ViewConfig};
use anyhow::Result;
use nu_ansi_term::Color;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use std::sync::LazyLock;

#[derive(Debug, Default, Clone)]
pub struct HelpCmd {}

impl HelpCmd {
    pub const NAME: &'static str = "help";
    pub fn view() -> Preview {
        Preview::new(&HELP_MESSAGE)
    }
}

static HELP_MESSAGE: LazyLock<String> = LazyLock::new(|| {
    let title = nu_ansi_term::Style::new().bold().underline();
    let code = nu_ansi_term::Style::new().bold().fg(Color::Blue);

    // There is probably a nicer way to do this formatting inline
    format!(
        r#"{}
Explore helps you dynamically navigate through your data!

{}
Launch Explore by piping data into it: {}

                   Move around:  Use the cursor keys
Drill down into records+tables:  Press <Enter> to select a cell, move around with cursor keys, press <Enter> again
            Go back/up a level:  Press <Esc> or "q"
 Transpose (flip rows+columns):  Press "t"
 Expand (show all nested data):  Press "e"
          Open this help page :  Type ":help" then <Enter>
      Open an interactive REPL:  Type ":try" then <Enter>
                     Scroll up:  Press "Page Up", Ctrl+B, or Alt+V
                   Scroll down:  Press "Page Down", Ctrl+F, or Ctrl+V
                  Exit Explore:  Type ":q" then <Enter>, or Ctrl+D. Alternately, press <Esc> or "q" until Explore exits

{}
Most commands support search via regular expressions.

You can type "/" and type a pattern you want to search on. Then hit <Enter> and you will see the search results.

To go to the next hit use "<n>" key. You also can do a reverse search by using "?" instead of "/".
"#,
        title.paint("Explore"),
        title.paint("Basics"),
        code.paint("ls | explore"),
        title.paint("Search")
    )
});

// TODO: search help could use some updating... search results get shown immediately after typing, don't need to press Enter
// const HELP_MESSAGE: &str = r#"# Explore

// Explore helps you dynamically navigate through your data

// ## Basics

//                            Move around:  Use the cursor keys
//         Drill down into records+tables:  Press <Enter> to select a cell, move around with cursor keys, then press <Enter> again
//                     Go back/up a level:  Press <Esc>
// Transpose data (flip rows and columns):  Press "t"
//     Expand data (show all nested data):  Press "e"
//                   Open this help page :  Type ":help" then <Enter>
//               Open an interactive REPL:  Type ":try" then <Enter>
//                         Scroll up/down:  Use the "Page Up" and "Page Down" keys
//                           Exit Explore:  Type ":q" then <Enter>, or Ctrl+D. Alternately, press <Esc> until Explore exits

// ## Search

// Most commands support search via regular expressions.

// You can type "/" and type a pattern you want to search on.
// Then hit <Enter> and you will see the search results.

// To go to the next hit use "<n>" key.

// You also can do a reverse search by using "?" instead of "/".
// "#;

impl ViewCommand for HelpCmd {
    type View = Preview;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn description(&self) -> &'static str {
        ""
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn spawn(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        _: Option<Value>,
        _: &ViewConfig,
    ) -> Result<Self::View> {
        Ok(HelpCmd::view())
    }
}
