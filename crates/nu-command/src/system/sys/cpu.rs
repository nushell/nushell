use super::trim_cstyle_null;
use nu_engine::command_prelude::*;
use sysinfo::{CpuRefreshKind, MINIMUM_CPU_UPDATE_INTERVAL, System};

#[derive(Clone)]
pub struct SysCpu;

impl Command for SysCpu {
    fn name(&self) -> &str {
        "sys cpu"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys cpu")
            .filter()
            .switch(
                "long",
                "Get all available columns (slower, needs to sample CPU over time)",
                Some('l'),
            )
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn description(&self) -> &str {
        "View information about the system CPUs."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let long = call.has_flag(engine_state, stack, "long")?;
        Ok(cpu(long, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Show info about the system CPUs",
            example: "sys cpu",
            result: None,
        }]
    }
}

fn cpu(long: bool, span: Span) -> Value {
    let mut sys = System::new();
    if long {
        sys.refresh_cpu_specifics(CpuRefreshKind::everything());
        // We must refresh the CPU twice a while apart to get valid usage data.
        // In theory we could just sleep MINIMUM_CPU_UPDATE_INTERVAL, but I've noticed that
        // that gives poor results (error of ~5%). Decided to wait 2x that long, somewhat arbitrarily
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL * 2);
        sys.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
    } else {
        sys.refresh_cpu_specifics(CpuRefreshKind::nothing().with_frequency());
    }

    let cpus = sys
        .cpus()
        .iter()
        .map(|cpu| {
            let load_avg = System::load_average();
            let load_avg = format!(
                "{:.2}, {:.2}, {:.2}",
                load_avg.one, load_avg.five, load_avg.fifteen
            );

            let mut record = record! {
                "name" => Value::string(trim_cstyle_null(cpu.name()), span),
                "brand" => Value::string(trim_cstyle_null(cpu.brand()), span),
                "vendor_id" => Value::string(trim_cstyle_null(cpu.vendor_id()), span),
                "freq" => Value::int(cpu.frequency() as i64, span),
                "load_average" => Value::string(load_avg, span),
            };

            if long {
                // sysinfo CPU usage numbers are not very precise unless you wait a long time between refreshes.
                // Round to 1DP (chosen somewhat arbitrarily) so people aren't misled by high-precision floats.
                let rounded_usage = (f64::from(cpu.cpu_usage()) * 10.0).round() / 10.0;
                record.push("cpu_usage", rounded_usage.into_value(span));
            }

            Value::record(record, span)
        })
        .collect();

    Value::list(cpus, span)
}
