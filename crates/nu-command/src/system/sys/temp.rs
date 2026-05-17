use nu_engine::command_prelude::*;
use sysinfo::Components;

#[derive(Clone)]
pub struct SysTemp;

impl Command for SysTemp {
    fn name(&self) -> &str {
        "sys temp"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys temp")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn description(&self) -> &str {
        "View the temperatures of system components."
    }

    fn extra_description(&self) -> &str {
        "Some system components do not support temperature readings, so this command may return an empty list if no components support temperature."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(temp(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Show the system temperatures",
            example: "sys temp",
            result: None,
        }]
    }
}

fn temp(span: Span) -> Value {
    let components = Components::new_with_refreshed_list()
        .iter()
        .map(|component| {
            let mut record = record! {
                "unit" => Value::string(component.label(), span),
                "temp" => Value::float(component.temperature().unwrap_or(f32::NAN).into(), span),
                "high" => Value::float(component.max().unwrap_or(f32::NAN).into(), span),
            };

            if let Some(critical) = component.critical() {
                record.push("critical", Value::float(critical.into(), span));
            }

            Value::record(record, span)
        })
        .collect();

    Value::list(components, span)
}
