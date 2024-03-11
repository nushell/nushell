use crate::query_json::QueryJson;
use crate::query_web::QueryWeb;
use crate::query_xml::QueryXml;

use nu_engine::documentation::get_flags_section;
use nu_plugin::{EvaluatedCall, LabeledError, Plugin, PluginCommand, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, Value};
use std::fmt::Write;

#[derive(Default)]
pub struct Query;

impl Query {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn usage() -> &'static str {
        "Usage: query"
    }
}

impl Plugin for Query {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(QueryCommand),
            Box::new(QueryJson),
            Box::new(QueryXml),
            Box::new(QueryWeb),
        ]
    }
}

// With no subcommand
pub struct QueryCommand;

impl SimplePluginCommand for QueryCommand {
    type Plugin = Query;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("query")
            .usage("Show all the query commands")
            .category(Category::Filters)
    }

    fn run(
        &self,
        _plugin: &Query,
        _engine: &nu_plugin::EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let help = get_brief_subcommand_help();
        Ok(Value::string(help, call.head))
    }
}

pub fn get_brief_subcommand_help() -> String {
    let sigs: Vec<_> = Query
        .commands()
        .into_iter()
        .map(|cmd| cmd.signature())
        .collect();

    let mut help = String::new();
    let _ = write!(help, "{}\n\n", sigs[0].sig.usage);
    let _ = write!(help, "Usage:\n  > {}\n\n", sigs[0].sig.name);
    help.push_str("Subcommands:\n");

    for x in sigs.iter().enumerate() {
        if x.0 == 0 {
            continue;
        }
        let _ = writeln!(help, "  {} - {}", x.1.sig.name, x.1.sig.usage);
    }

    help.push_str(&get_flags_section(None, &sigs[0].sig, |v| {
        format!("{:#?}", v)
    }));
    help
}
