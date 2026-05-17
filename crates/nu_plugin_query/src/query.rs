use crate::{
    query_json::QueryJson, query_web::QueryWeb, query_webpage_info::QueryWebpageInfo,
    query_xml::QueryXml,
};
use nu_plugin::{EvaluatedCall, Plugin, PluginCommand, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, Value};

#[derive(Default)]
pub struct Query;

impl Query {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Plugin for Query {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(QueryCommand),
            Box::new(QueryJson),
            Box::new(QueryXml),
            Box::new(QueryWeb),
            Box::new(QueryWebpageInfo),
        ]
    }
}

// With no subcommand
pub struct QueryCommand;

impl SimplePluginCommand for QueryCommand {
    type Plugin = Query;

    fn name(&self) -> &str {
        "query"
    }

    fn description(&self) -> &str {
        "Show all the query commands"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self)).category(Category::Filters)
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
