use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Type};
use polars_plan::prelude::{DataTypeSelector, Selector, TimeUnitSet};

#[derive(Clone)]
pub struct SelectorDuration;

impl PluginCommand for SelectorDuration {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector duration"
    }

    fn description(&self) -> &str {
        "Select all duration columns. Optionally filter by time unit (ns, us, ms)."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "time-unit",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Filter by time unit(s): ns (nanoseconds), us (microseconds), ms (milliseconds).",
                None,
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "polars selector duration",
                description: "Create a selector for all duration columns",
                result: None,
            },
            Example {
                example: "polars selector duration --time-unit [ns us]",
                description: "Create a selector for nanosecond or microsecond duration columns",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "duration", "interval", "time"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();

        let time_units: Vec<String> = call
            .get_flag::<Vec<String>>("time-unit")?
            .unwrap_or_default();

        let tu_set = parse_time_unit_set(&time_units, call)?;

        let selector = Selector::ByDType(DataTypeSelector::Duration(tu_set));
        let nu_selector = NuSelector::from(selector);

        nu_selector
            .to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn parse_time_unit_set(
    time_units: &[String],
    call: &EvaluatedCall,
) -> Result<TimeUnitSet, LabeledError> {
    if time_units.is_empty() {
        return Ok(TimeUnitSet::all());
    }
    let mut set = TimeUnitSet::empty();
    for tu in time_units {
        let flag = match tu.as_str() {
            "ns" => TimeUnitSet::NANO_SECONDS,
            "us" => TimeUnitSet::MICRO_SECONDS,
            "ms" => TimeUnitSet::MILLI_SECONDS,
            other => {
                return Err(LabeledError::new(format!("Invalid time unit: '{other}'"))
                    .with_label("expected 'ns', 'us', or 'ms'", call.head));
            }
        };
        set |= flag;
    }
    Ok(set)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), nu_protocol::ShellError> {
        test_polars_plugin_command(&SelectorDuration)
    }
}
