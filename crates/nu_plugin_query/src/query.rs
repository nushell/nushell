use crate::query_json::QueryJson;
use crate::query_web::QueryWeb;
use crate::query_xml::QueryXml;

use nu_plugin::{EvaluatedCall, Plugin, PluginCommand, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, PluginSignature, Value};

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
        engine: &nu_plugin::EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        Ok(Value::string(engine.get_help()?, call.head))
    }
}
