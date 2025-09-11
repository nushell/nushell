use super::trim_cstyle_null;
use chrono::{DateTime, FixedOffset, Local};
use nu_engine::command_prelude::*;
use sysinfo::System;

#[derive(Clone)]
pub struct SysHost;

impl Command for SysHost {
    fn name(&self) -> &str {
        "sys host"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys host")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::record())])
    }

    fn description(&self) -> &str {
        "View information about the system host."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(host(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Show info about the system host",
            example: "sys host",
            result: None,
        }]
    }
}

fn host(span: Span) -> Value {
    let mut record = Record::new();

    if let Some(name) = System::name() {
        record.push("name", Value::string(trim_cstyle_null(name), span));
    }
    if let Some(version) = System::os_version() {
        record.push("os_version", Value::string(trim_cstyle_null(version), span));
    }
    if let Some(long_version) = System::long_os_version() {
        record.push(
            "long_os_version",
            Value::string(trim_cstyle_null(long_version), span),
        );
    }
    if let Some(version) = System::kernel_version() {
        record.push(
            "kernel_version",
            Value::string(trim_cstyle_null(version), span),
        );
    }
    if let Some(hostname) = System::host_name() {
        record.push("hostname", Value::string(trim_cstyle_null(hostname), span));
    }

    let uptime = System::uptime()
        .saturating_mul(1_000_000_000)
        .try_into()
        .unwrap_or(i64::MAX);

    record.push("uptime", Value::duration(uptime, span));

    let boot_time = boot_time()
        .map(|time| Value::date(time, span))
        .unwrap_or(Value::nothing(span));

    record.push("boot_time", boot_time);

    Value::record(record, span)
}

fn boot_time() -> Option<DateTime<FixedOffset>> {
    // Broken systems can apparently return really high values.
    // See: https://github.com/nushell/nushell/issues/10155
    // First, try to convert u64 to i64, and then try to create a `DateTime`.
    let secs = System::boot_time().try_into().ok()?;
    let time = DateTime::from_timestamp(secs, 0)?;
    Some(time.with_timezone(&Local).fixed_offset())
}
