use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Type};
use polars::prelude::TimeZone;
use polars_plan::prelude::{DataTypeSelector, Selector, TimeUnitSet, TimeZoneSet};
use std::sync::Arc;

#[derive(Clone)]
pub struct SelectorDatetime;

impl PluginCommand for SelectorDatetime {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector datetime"
    }

    fn description(&self) -> &str {
        r#"Select all datetime columns. Optionally filter by time unit (ns, us, ms) and/or timezone."#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "time-unit",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Filter by time unit(s): ns (nanoseconds), us (microseconds), ms (milliseconds).",
                None,
            )
            .named(
                "time-zone",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                r#"Filter by timezone(s). Use "*" to match any set timezone, or "unset" to match columns without a timezone."#,
                None,
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "polars selector datetime",
                description: "Create a selector for all datetime columns",
                result: None,
            },
            Example {
                example: "polars selector datetime --time-unit [ns us]",
                description: "Create a selector for nanosecond or microsecond datetime columns",
                result: None,
            },
            Example {
                example: r#"polars selector datetime --time-unit [ns] --time-zone [UTC]"#,
                description: "Create a selector for nanosecond datetime columns with UTC timezone",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "datetime", "timestamp", "time"]
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
        let time_zones: Vec<String> = call
            .get_flag::<Vec<String>>("time-zone")?
            .unwrap_or_default();

        let tu_set = parse_time_unit_set(&time_units, call)?;
        let tz_set = parse_time_zone_set(&time_zones, call)?;

        let selector = Selector::ByDType(DataTypeSelector::Datetime(tu_set, tz_set));
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

fn parse_time_zone_set(
    time_zones: &[String],
    call: &EvaluatedCall,
) -> Result<TimeZoneSet, LabeledError> {
    if time_zones.is_empty() {
        return Ok(TimeZoneSet::Any);
    }
    if time_zones.len() == 1 && time_zones[0] == "*" {
        return Ok(TimeZoneSet::Any);
    }
    if time_zones.len() == 1 && time_zones[0] == "unset" {
        return Ok(TimeZoneSet::Unset);
    }
    let mut parsed: Vec<TimeZone> = Vec::with_capacity(time_zones.len());
    for tz_str in time_zones {
        let tz = TimeZone::opt_try_new(Some(tz_str.as_str()))
            .map_err(|e| {
                LabeledError::new(format!("Invalid timezone '{tz_str}': {e}"))
                    .with_label("invalid timezone string", call.head)
            })?
            .ok_or_else(|| {
                LabeledError::new("Empty timezone string")
                    .with_label("timezone cannot be empty", call.head)
            })?;
        parsed.push(tz);
    }
    let arc: Arc<[TimeZone]> = parsed.into();
    Ok(TimeZoneSet::AnyOf(arc))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), nu_protocol::ShellError> {
        test_polars_plugin_command(&SelectorDatetime)
    }
}
