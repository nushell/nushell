use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, ShellError, Signature};

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuSelector, PolarsPluginType},
};

pub struct SelectorNot;

impl PluginCommand for SelectorNot {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector not"
    }

    fn description(&self) -> &str {
        "Inverts selector."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(
                PolarsPluginType::NuSelector.into(),
                PolarsPluginType::NuSelector.into(),
            )])
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Inverts selector",
            example: "polars selector first | polars selector not",
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        NuSelector::try_from_pipeline(plugin, input, call.head)
            .and_then(|s| command(plugin, engine, call, s))
            .map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    selector: NuSelector,
) -> Result<PipelineData, ShellError> {
    let result: NuSelector = (!selector.into_polars()).into();
    result.to_pipeline_data(plugin, engine, call.head)
}
