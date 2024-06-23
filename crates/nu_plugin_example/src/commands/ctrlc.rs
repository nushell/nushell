use std::sync::{Arc, Barrier};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature};

use crate::ExamplePlugin;

/// `example ctrlc`
pub struct Ctrlc;

impl PluginCommand for Ctrlc {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example ctrlc"
    }

    fn usage(&self) -> &str {
        "Example command that demonstrates registering a ctrl-c handler"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example"]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        engine: &EngineInterface,
        _call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let barrier = Arc::new(Barrier::new(2));

        let _guard = engine.register_ctrlc_handler(Box::new({
            let barrier = barrier.clone();
            move || {
                barrier.wait();
            }
        }));

        eprintln!("waiting for ctrl-c signal...");
        barrier.wait();
        eprintln!("peace.");

        Ok(PipelineData::Empty)
    }
}
