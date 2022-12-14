use std::{
    collections::HashMap,
    io::{self, Result},
};

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::{nu_common::NuSpan, pager::TableConfig, views::RecordView};

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
    type View = RecordView<'static>;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        Some(HelpManual {
            name: "help",
            description: "Explore the help page for `explore`",
            arguments: vec![],
            examples: vec![
                HelpExample {
                    example: "help",
                    description: "Open the help page for all of `explore`",
                },
                HelpExample {
                    example: "help nu",
                    description: "Open the help page for the `nu` explore command",
                },
                HelpExample {
                    example: "help help",
                    description: "...It was supposed to be hidden....until...now...",
                },
            ],
        })
    }

    fn parse(&mut self, args: &str) -> Result<()> {
        self.input_command = args.trim().to_owned();

        Ok(())
    }

    fn spawn(&mut self, _: &EngineState, _: &mut Stack, _: Option<Value>) -> Result<Self::View> {
        if self.input_command.is_empty() {
            let (headers, data) = help_frame_data(&self.supported_commands, &self.aliases);
            let view = RecordView::new(headers, data, self.table_cfg);
            return Ok(view);
        }

        let manual = self
            .supported_commands
            .iter()
            .find(|manual| manual.name == self.input_command)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "a given command was not found"))?;

        let aliases = self
            .aliases
            .get(manual.name)
            .map(|l| l.as_slice())
            .unwrap_or(&[]);
        let (headers, data) = help_manual_data(manual, aliases);
        let view = RecordView::new(headers, data, self.table_cfg);

        Ok(view)
    }
}

fn help_frame_data(
    supported_commands: &[HelpManual],
    aliases: &HashMap<String, Vec<String>>,
) -> (Vec<String>, Vec<Vec<Value>>) {
    macro_rules! null {
        () => {
            Value::Nothing {
                span: NuSpan::unknown(),
            }
        };
    }

    macro_rules! nu_str {
        ($text:expr) => {
            Value::string($text.to_string(), NuSpan::unknown())
        };
    }

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

    let headers = vec!["name", "mode", "information", "description"];

    #[rustfmt::skip]
    let shortcuts = [
        (":",      "view",    commands,  "Run an explore command (explore the 'information' cell of this row to list commands)"),
        ("/",      "view",    null!(),   "Search for a pattern"),
        ("?",      "view",    null!(),   "Search for a pattern, but the <n> key now scrolls to the previous result"),
        ("n",      "view",    null!(),   "When searching, scroll to the next search result"),
        ("i",      "view",    null!(),   "Enters cursor mode to inspect individual cells"),
        ("t",      "view",    null!(),   "Transpose table, so that columns become rows and vice versa"),
        ("Up",     "",        null!(),   "Moves the cursor or viewport one row up"),
        ("Down",   "",        null!(),   "Moves the cursor or viewport one row down"),
        ("Left",   "",        null!(),   "Moves the cursor or viewport one column left"),
        ("Right",  "",        null!(),   "Moves the cursor or viewport one column right"),
        ("PgDown", "view",    null!(),   "Moves the cursor or viewport one page of rows down"),
        ("PgUp",   "view",    null!(),   "Moves the cursor or viewport one page of rows up"),
        ("Esc",    "",        null!(),   "Exits cursor mode. Exits the currently explored data."),
        ("Enter",  "cursor",  null!(),   "In cursor mode, explore the data of the selected cell"),
    ];

    let headers = headers.iter().map(|s| s.to_string()).collect();
    let data = shortcuts
        .iter()
        .map(|(name, mode, info, desc)| {
            vec![nu_str!(name), nu_str!(mode), info.clone(), nu_str!(desc)]
        })
        .collect();

    (headers, data)
}

fn help_manual_data(manual: &HelpManual, aliases: &[String]) -> (Vec<String>, Vec<Vec<Value>>) {
    macro_rules! nu_str {
        ($text:expr) => {
            Value::string($text, NuSpan::unknown())
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

    let name = nu_str!(manual.name);
    let aliases = nu_str!(aliases.join(", "));
    let desc = nu_str!(manual.description);

    let headers = vec![
        String::from("name"),
        String::from("aliases"),
        String::from("arguments"),
        String::from("examples"),
        String::from("description"),
    ];

    let data = vec![vec![name, aliases, arguments, examples, desc]];

    (headers, data)
}
