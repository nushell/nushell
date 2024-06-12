use super::trim_cstyle_null;
use nu_engine::command_prelude::*;
use sysinfo::{CpuRefreshKind, System, MINIMUM_CPU_UPDATE_INTERVAL};

#[derive(Clone)]
pub struct SysCpu;

impl Command for SysCpu {
    fn name(&self) -> &str {
        "sys cpu"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys cpu")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn usage(&self) -> &str {
        "View information about the system CPUs."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(cpu(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show info about the system CPUs",
            example: "sys cpu",
            result: None,
        }]
    }
}

fn cpu(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_cpu_specifics(CpuRefreshKind::everything());
    // We must refresh the CPU twice a while apart to get valid usage data.
    // In theory we could just sleep MINIMUM_CPU_UPDATE_INTERVAL, but I've noticed that
    // that gives poor results (error of ~5%). Decided to wait 2x that long, somewhat arbitrarily
    std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL * 2);
    sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());

    let cpus = sys
        .cpus()
        .iter()
        .map(|cpu| {
            // sysinfo CPU usage numbers are not very precise unless you wait a long time between refreshes.
            // Round to 1DP (chosen somewhat arbitrarily) so people aren't misled by high-precision floats.
            let rounded_usage = (cpu.cpu_usage() * 10.0).round() / 10.0;

            let load_avg = System::load_average();
            let load_avg = format!(
                "{:.2}, {:.2}, {:.2}",
                load_avg.one, load_avg.five, load_avg.fifteen
            );

            let record = record! {
                "name" => Value::string(trim_cstyle_null(cpu.name()), span),
                "brand" => Value::string(trim_cstyle_null(cpu.brand()), span),
                "freq" => Value::int(cpu.frequency() as i64, span),
                "cpu_usage" => Value::float(rounded_usage.into(), span),
                "load_average" => Value::string(load_avg, span),
                "vendor_id" => Value::string(trim_cstyle_null(cpu.vendor_id()), span),
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(cpus, span)
}
