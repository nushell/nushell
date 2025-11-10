use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, DynamicSuggestion, Example, LabeledError, PipelineData, Signature, SyntaxShape,
    engine::ArgType,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ExamplePlugin;

/// `<list> | example sum`
pub struct ArgCompletion;

impl PluginCommand for ArgCompletion {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example arg-completion"
    }

    fn description(&self) -> &str {
        "It's a demo for arg completion, you can try to type `example arg-completion -f <tab>`to see what it returns"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional("second", SyntaxShape::String, "optional second")
            .required("first", SyntaxShape::String, "required integer value")
            .named(
                "future-timestamp",
                SyntaxShape::Int,
                "example flag which support auto completion",
                Some('f'),
            )
            .category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        _engine: &EngineInterface,
        _call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        Ok(PipelineData::empty())
    }

    fn get_dynamic_completion(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        arg_type: ArgType,
    ) -> Option<Vec<DynamicSuggestion>> {
        match arg_type {
            ArgType::Flag(flag_name) => {
                // let's generate it dynamically.
                let start = SystemTime::now();
                let since_the_epoch = start
                    .duration_since(UNIX_EPOCH)
                    .expect("time should go forward")
                    .as_secs();
                match flag_name.as_ref() {
                    "future-timestamp" => Some(
                        (since_the_epoch..since_the_epoch + 10)
                            .map(|s| DynamicSuggestion {
                                value: s.to_string(),
                                ..Default::default()
                            })
                            .collect(),
                    ),
                    _ => None,
                }
            }
            ArgType::Positional(index) => {
                // let's generate it dynamically too
                let start = SystemTime::now();
                let since_the_epoch = start
                    .duration_since(UNIX_EPOCH)
                    .expect("time should go forward")
                    .as_secs();
                // be careful: Don't include any spaces for values.
                if index == 0 {
                    Some(
                        (since_the_epoch..since_the_epoch + 10)
                            .map(|s| DynamicSuggestion {
                                value: format!("arg0:{s}"),
                                ..Default::default()
                            })
                            .collect(),
                    )
                } else if index == 1 {
                    Some(
                        (since_the_epoch..since_the_epoch + 10)
                            .map(|s| DynamicSuggestion {
                                value: format!("arg1:{s}"),
                                ..Default::default()
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            }
        }
    }
}
