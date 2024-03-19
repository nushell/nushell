use nu_plugin::{
    EngineInterface, EvaluatedCall, LabeledError, Plugin, PluginCommand, SimplePluginCommand,
};
use nu_protocol::{Category, PluginSignature, Value};

mod commands;
pub use commands::*;

pub struct StreamExample;

impl Plugin for StreamExample {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(Main),
            Box::new(Seq),
            Box::new(Sum),
            Box::new(CollectExternal),
            Box::new(ForEach),
            Box::new(Generate),
        ]
    }
}

/// `stream_example`
pub struct Main;

impl SimplePluginCommand for Main {
    type Plugin = StreamExample;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("stream_example")
            .usage("Examples for streaming plugins")
            .search_terms(vec!["example".into()])
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _plugin: &StreamExample,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        Err(LabeledError {
            label: "No subcommand provided".into(),
            msg: "add --help here to see usage".into(),
            span: Some(call.head.past()),
        })
    }
}
