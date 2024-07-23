use std::sync::mpsc;

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
        let (sender, receiver) = mpsc::channel::<()>();
        let _guard = engine.register_ctrlc_handler(Box::new(move || {
            let _ = sender.send(());
        }));

        eprintln!("waiting for ctrl-c signal...");
        receiver.recv().expect("handler went away");
        eprintln!("interrupt status: {:?}", engine.signals().interrupted());
        eprintln!("peace.");

        Ok(PipelineData::Empty)
    }
}
