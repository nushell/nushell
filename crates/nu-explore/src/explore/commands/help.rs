use super::super::views::{Preview, ViewConfig};
use super::ViewCommand;
use anyhow::Result;
use nu_ansi_term::Color;
use nu_protocol::{
    Value,
    engine::{EngineState, Stack},
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
    let title = nu_ansi_term::Style::new().bold();
    let section = nu_ansi_term::Style::new().bold().fg(Color::Cyan);
    let code = nu_ansi_term::Style::new().bold().fg(Color::Blue);
    let key = nu_ansi_term::Style::new().bold().fg(Color::Green);
    let dim = nu_ansi_term::Style::new().dimmed();

    format!(
        r#"
  {} Explore Help {}

  Explore helps you dynamically navigate through your data.
  Launch it by piping data into the command: {}

  {} Navigation

    {}            Move cursor up/down/left/right
    {}              Drill into a cell (select it)
    {}            Go back / exit current view
    {}        Page up / Page down

  {} Data Manipulation

    {}                  Transpose (flip rows and columns)
    {}                  Expand (show all nested data)

  {} Commands {}

    {}              Show this help page
    {}               Open interactive REPL
    {}          Run a Nushell command on current data
    {}                 Exit Explore

  {} Search

    {}                  Start forward search
    {}                  Start reverse search
    {} {} {}          Navigate search results

"#,
        title.paint("━━"),
        title.paint("━━"),
        code.paint("ls | explore"),
        section.paint("▸"),
        key.paint("↑ ↓ ← →"),
        key.paint("Enter"),
        key.paint("Esc / q"),
        key.paint("PgUp / PgDn"),
        section.paint("▸"),
        key.paint("t"),
        key.paint("e"),
        section.paint("▸"),
        dim.paint("(type : then command)"),
        key.paint(":help"),
        key.paint(":try"),
        key.paint(":nu <cmd>"),
        key.paint(":q"),
        section.paint("▸"),
        key.paint("/"),
        key.paint("?"),
        key.paint("n"),
        key.paint("N"),
        key.paint("Enter"),
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
