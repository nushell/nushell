use std::{
    collections::HashMap,
    io::{self, Result},
};

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::{
    nu_common::{collect_input, NuSpan},
    pager::TableConfig,
    views::{Preview, RecordView, View},
};

use super::{HelpExample, HelpManual, ViewCommand};

#[derive(Debug, Default, Clone)]
pub struct HelpCmd {
    input_command: String,
    table_cfg: TableConfig,
    supported_commands: Vec<HelpManual>,
    aliases: HashMap<String, Vec<String>>,
}

impl HelpCmd {
    pub const NAME: &'static str = "help";

    const HELP_MESSAGE: &'static str = r#"                        Explore - main help file

          Move around:  Use the cursor keys.
    Close this window:  Use "<Esc>".
   Get out of Explore:  Use ":q<Enter>" (or <Ctrl> + <D>).

   Get specific help:   It is possible to go directly to whatewer you want help on,
                        by giving an argument to the ":help" command.
                        
                        Currently you can get only help on a different commands.
                        To obtain a list of supported commands run ":help :<Enter>"

------------------------------------------------------------------------------------

Regular expressions ~

Most commands you can use support regular expressions.

You can type "/" and type a pattern you wanna search on.
Then hit <Enter> and you are going to see the search results.

To jump over them use "<n>" key.

You also can make a reverse search by using "?" instead of "/".
"#;

    pub fn new(
        commands: Vec<HelpManual>,
        aliases: &[(&str, &str)],
        table_cfg: TableConfig,
    ) -> Self {
        let aliases = collect_aliases(aliases);

        Self {
            input_command: String::new(),
            supported_commands: commands,
            aliases,
            table_cfg,
        }
    }
}

fn collect_aliases(aliases: &[(&str, &str)]) -> HashMap<String, Vec<String>> {
    let mut out_aliases: HashMap<String, Vec<String>> = HashMap::new();
    for (name, cmd) in aliases {
        out_aliases
            .entry(cmd.to_string())
            .and_modify(|list| list.push(name.to_string()))
            .or_insert_with(|| vec![name.to_string()]);
    }
    out_aliases
}

impl ViewCommand for HelpCmd {
    type View = HelpView<'static>;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        #[rustfmt::skip]
        let examples = vec![
            HelpExample::new("help",        "Open the help page for all of `explore`"),
            HelpExample::new("help :nu",     "Open the help page for the `nu` explore command"),
            HelpExample::new("help :help",   "...It was supposed to be hidden....until...now..."),
        ];

        #[rustfmt::skip]
        let arguments = vec![
            HelpExample::new("help :command", "you can provide a command and a help information for it will be displayed")
        ];

        Some(HelpManual {
            name: "help",
            description: "Explore the help page for `explore`",
            arguments,
            examples,
            input: vec![],
        })
    }

    fn parse(&mut self, args: &str) -> Result<()> {
        self.input_command = args.trim().to_owned();

        Ok(())
    }

    fn spawn(&mut self, _: &EngineState, _: &mut Stack, _: Option<Value>) -> Result<Self::View> {
        if self.input_command.is_empty() {
            return Ok(HelpView::Preview(Preview::new(Self::HELP_MESSAGE)));
        }

        if !self.input_command.starts_with(':') {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "unexpected help argument",
            ));
        }

        if self.input_command == ":" {
            let (headers, data) = help_frame_data(&self.supported_commands, &self.aliases);
            let view = RecordView::new(headers, data, self.table_cfg);
            return Ok(HelpView::Records(view));
        }

        let command = self
            .input_command
            .strip_prefix(':')
            .expect("we just checked the prefix");

        let manual = self
            .supported_commands
            .iter()
            .find(|manual| manual.name == command)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "a given command was not found"))?;

        let aliases = self
            .aliases
            .get(manual.name)
            .map(|l| l.as_slice())
            .unwrap_or(&[]);
        let (headers, data) = help_manual_data(manual, aliases);
        let view = RecordView::new(headers, data, self.table_cfg);

        Ok(HelpView::Records(view))
    }
}

fn help_frame_data(
    supported_commands: &[HelpManual],
    aliases: &HashMap<String, Vec<String>>,
) -> (Vec<String>, Vec<Vec<Value>>) {
    let commands = supported_commands
        .iter()
        .map(|manual| {
            let aliases = aliases
                .get(manual.name)
                .map(|l| l.as_slice())
                .unwrap_or(&[]);

            let (cols, mut vals) = help_manual_data(manual, aliases);
            let vals = vals.remove(0);
            Value::Record {
                cols,
                vals,
                span: NuSpan::unknown(),
            }
        })
        .collect();
    let commands = Value::List {
        vals: commands,
        span: NuSpan::unknown(),
    };

    collect_input(commands)
}

fn help_manual_data(manual: &HelpManual, aliases: &[String]) -> (Vec<String>, Vec<Vec<Value>>) {
    macro_rules! nu_str {
        ($text:expr) => {
            Value::String {
                val: $text.to_string(),
                span: NuSpan::unknown(),
            }
        };
    }

    let arguments = manual
        .arguments
        .iter()
        .map(|e| Value::Record {
            cols: vec![String::from("example"), String::from("description")],
            vals: vec![nu_str!(e.example), nu_str!(e.description)],
            span: NuSpan::unknown(),
        })
        .collect();

    let arguments = Value::List {
        vals: arguments,
        span: NuSpan::unknown(),
    };

    let examples = manual
        .examples
        .iter()
        .map(|e| Value::Record {
            cols: vec![String::from("example"), String::from("description")],
            vals: vec![nu_str!(e.example), nu_str!(e.description)],
            span: NuSpan::unknown(),
        })
        .collect();
    let examples = Value::List {
        vals: examples,
        span: NuSpan::unknown(),
    };

    let inputs = manual
        .input
        .iter()
        .map(|e| Value::Record {
            cols: vec![
                String::from("name"),
                String::from("context"),
                String::from("description"),
            ],
            vals: vec![nu_str!(e.code), nu_str!(e.context), nu_str!(e.description)],
            span: NuSpan::unknown(),
        })
        .collect();
    let inputs = Value::List {
        vals: inputs,
        span: NuSpan::unknown(),
    };

    let name = nu_str!(manual.name);
    let aliases = nu_str!(aliases.join(", "));
    let desc = nu_str!(manual.description);

    let headers = vec![
        String::from("name"),
        String::from("aliases"),
        String::from("arguments"),
        String::from("input"),
        String::from("examples"),
        String::from("description"),
    ];

    let data = vec![vec![name, aliases, arguments, inputs, examples, desc]];

    (headers, data)
}
pub enum HelpView<'a> {
    Records(RecordView<'a>),
    Preview(Preview),
}

impl View for HelpView<'_> {
    fn draw(
        &mut self,
        f: &mut crate::pager::Frame,
        area: tui::layout::Rect,
        cfg: &crate::ViewConfig,
        layout: &mut crate::views::Layout,
    ) {
        match self {
            HelpView::Records(v) => v.draw(f, area, cfg, layout),
            HelpView::Preview(v) => v.draw(f, area, cfg, layout),
        }
    }

    fn handle_input(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        layout: &crate::views::Layout,
        info: &mut crate::pager::ViewInfo,
        key: crossterm::event::KeyEvent,
    ) -> Option<crate::pager::Transition> {
        match self {
            HelpView::Records(v) => v.handle_input(engine_state, stack, layout, info, key),
            HelpView::Preview(v) => v.handle_input(engine_state, stack, layout, info, key),
        }
    }

    fn show_data(&mut self, i: usize) -> bool {
        match self {
            HelpView::Records(v) => v.show_data(i),
            HelpView::Preview(v) => v.show_data(i),
        }
    }

    fn collect_data(&self) -> Vec<crate::nu_common::NuText> {
        match self {
            HelpView::Records(v) => v.collect_data(),
            HelpView::Preview(v) => v.collect_data(),
        }
    }

    fn exit(&mut self) -> Option<Value> {
        match self {
            HelpView::Records(v) => v.exit(),
            HelpView::Preview(v) => v.exit(),
        }
    }
}
