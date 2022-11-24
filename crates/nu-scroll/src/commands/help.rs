use std::io::{self, Result};

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
}

impl HelpCmd {
    pub const NAME: &'static str = "help";

    pub fn new(commands: Vec<HelpManual>, table_cfg: TableConfig) -> Self {
        Self {
            input_command: String::new(),
            supported_commands: commands,
            table_cfg,
        }
    }
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
            description: "Looks up a help information about a command or a `scroll`",
            arguments: vec![],
            examples: vec![
                HelpExample {
                    example: "help",
                    description: "Open a help information about the `scroll`",
                },
                HelpExample {
                    example: "help nu",
                    description: "Find a help list of `nu` command",
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
            let (headers, data) = help_frame_data(&self.supported_commands);
            let view = RecordView::new(headers, data, self.table_cfg.clone());
            return Ok(view);
        }

        let manual = self
            .supported_commands
            .iter()
            .find(|manual| manual.name == self.input_command)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "a given command was not found"))?;

        let (headers, data) = help_manual_data(manual);
        let view = RecordView::new(headers, data, self.table_cfg.clone());

        Ok(view)
    }
}

fn help_frame_data(supported_commands: &[HelpManual]) -> (Vec<String>, Vec<Vec<Value>>) {
    macro_rules! null {
        () => {
            Value::Nothing {
                span: NuSpan::unknown(),
            }
        };
    }

    macro_rules! nu_str {
        ($text:expr) => {
            Value::String {
                val: $text.to_string(),
                span: NuSpan::unknown(),
            }
        };
    }

    let commands = supported_commands
        .iter()
        .map(|manual| {
            let (cols, mut vals) = help_manual_data(manual);
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
        (":",      "view",    commands,  "Run a command"),
        ("/",      "view",    null!(),   "Search via pattern"),
        ("?",      "view",    null!(),   "Search via pattern but results will be reversed when you press <n>"),
        ("n",      "view",    null!(),   "Gets to the next found element in search"),
        ("i",      "view",    null!(),   "Turn on a cursor mode so you can inspect values"),
        ("t",      "view",    null!(),   "Transpose table, so columns became rows and vice versa"),
        ("Up",     "",        null!(),   "Moves to an element above"),
        ("Down",   "",        null!(),   "Moves to an element bellow"),
        ("Left",   "",        null!(),   "Moves to an element to the left"),
        ("Right",  "",        null!(),   "Moves to an element to the right"),
        ("PgDown", "view",    null!(),   "Moves to an a bunch of elements bellow"),
        ("PgUp",   "view",    null!(),   "Moves to an a bunch of elements above"),
        ("Esc",    "",        null!(),   "Exits a cursor mode. Exists an expected element."),
        ("Enter",  "cursor",  null!(),   "Inspect a chosen element"),
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

fn help_manual_data(manual: &HelpManual) -> (Vec<String>, Vec<Vec<Value>>) {
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

    let headers = vec![
        String::from("name"),
        String::from("arguments"),
        String::from("examples"),
        String::from("description"),
    ];
    let data = vec![vec![
        nu_str!(manual.name),
        arguments,
        examples,
        nu_str!(manual.description),
    ]];

    (headers, data)
}
