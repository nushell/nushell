use chrono::prelude::DateTime;
use chrono::Local;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    record, Category, Example, IntoPipelineData, LazyRecord, PipelineData, Record, ShellError,
    Signature, Span, Type, Value,
};
use std::time::{Duration, UNIX_EPOCH};
use sysinfo::{
    ComponentExt, CpuExt, CpuRefreshKind, DiskExt, NetworkExt, System, SystemExt, UserExt,
};

#[derive(Clone)]
pub struct Sys;

impl Command for Sys {
    fn name(&self) -> &str {
        "sys"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::Record(vec![]))])
    }

    fn usage(&self) -> &str {
        "View information about the system."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.span();
        let ret = Value::LazyRecord {
            val: Box::new(SysResult { span }),
            span,
        };

        Ok(ret.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Show info about the system",
                example: "sys",
                result: None,
            },
            Example {
                description: "Show the os system name with get",
                example: "(sys).host | get name",
                result: None,
            },
            Example {
                description: "Show the os system name",
                example: "(sys).host.name",
                result: None,
            },
        ]
    }
}

#[derive(Debug, Clone)]
pub struct SysResult {
    pub span: Span,
}

impl LazyRecord<'_> for SysResult {
    fn column_names(&self) -> Vec<&'static str> {
        vec!["host", "cpu", "disks", "mem", "temp", "net"]
    }

    fn get_column_value(&self, column: &str) -> Result<Value, ShellError> {
        let span = self.span;

        match column {
            "host" => Ok(host(span)),
            "cpu" => Ok(cpu(span)),
            "disks" => Ok(disks(span)),
            "mem" => Ok(mem(span)),
            "temp" => Ok(temp(span)),
            "net" => Ok(net(span)),
            _ => Err(ShellError::LazyRecordAccessFailed {
                message: format!("Could not find column '{column}'"),
                column_name: column.to_string(),
                span,
            }),
        }
    }

    fn span(&self) -> Span {
        self.span
    }

    fn clone_value(&self, span: Span) -> Value {
        Value::LazyRecord {
            val: Box::new((*self).clone()),
            span,
        }
    }
}

pub fn trim_cstyle_null(s: String) -> String {
    s.trim_matches(char::from(0)).to_string()
}

pub fn disks(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_disks();
    sys.refresh_disks_list();

    let mut output = vec![];
    for disk in sys.disks() {
        let device = trim_cstyle_null(disk.name().to_string_lossy().to_string());
        let typ = trim_cstyle_null(String::from_utf8_lossy(disk.file_system()).to_string());

        let record = record! {
            "device" => Value::string(device, span),
            "type" => Value::string(typ, span),
            "mount" => Value::string(disk.mount_point().to_string_lossy(), span),
            "total" => Value::filesize(disk.total_space() as i64, span),
            "free" => Value::filesize(disk.available_space() as i64, span),
            "removable" => Value::bool(disk.is_removable(), span),
            "kind" => Value::string(format!("{:?}", disk.kind()), span),
        };

        output.push(Value::record(record, span));
    }
    Value::List { vals: output, span }
}

pub fn net(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_networks();
    sys.refresh_networks_list();

    let mut output = vec![];
    for (iface, data) in sys.networks() {
        let record = record! {
            "name" => Value::string(trim_cstyle_null(iface.to_string()), span),
            "sent" => Value::filesize(data.total_transmitted() as i64, span),
            "recv" => Value::filesize(data.total_received() as i64, span),
        };

        output.push(Value::record(record, span));
    }
    Value::List { vals: output, span }
}

pub fn cpu(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_cpu_specifics(CpuRefreshKind::everything());
    // We must refresh the CPU twice a while apart to get valid usage data.
    // In theory we could just sleep MINIMUM_CPU_UPDATE_INTERVAL, but I've noticed that
    // that gives poor results (error of ~5%). Decided to wait 2x that long, somewhat arbitrarily
    std::thread::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL * 2);
    sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());

    let mut output = vec![];
    for cpu in sys.cpus() {
        // sysinfo CPU usage numbers are not very precise unless you wait a long time between refreshes.
        // Round to 1DP (chosen somewhat arbitrarily) so people aren't misled by high-precision floats.
        let rounded_usage = (cpu.cpu_usage() * 10.0).round() / 10.0;

        let load_avg = sys.load_average();
        let load_avg = trim_cstyle_null(format!(
            "{:.2}, {:.2}, {:.2}",
            load_avg.one, load_avg.five, load_avg.fifteen
        ));

        let record = record! {
            "name" => Value::string(trim_cstyle_null(cpu.name().to_string()), span),
            "brand" => Value::string(trim_cstyle_null(cpu.brand().to_string()), span),
            "freq" => Value::int(cpu.frequency() as i64, span),
            "cpu_usage" => Value::float(rounded_usage as f64, span),
            "load_average" => Value::string(load_avg, span),
            "vendor_id" => Value::string(trim_cstyle_null(cpu.vendor_id().to_string()), span),
        };

        output.push(Value::record(record, span));
    }

    Value::List { vals: output, span }
}

pub fn mem(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_memory();

    let total_mem = sys.total_memory();
    let free_mem = sys.free_memory();
    let used_mem = sys.used_memory();
    let avail_mem = sys.available_memory();

    let total_swap = sys.total_swap();
    let free_swap = sys.free_swap();
    let used_swap = sys.used_swap();

    let record = record! {
        "total" => Value::filesize(total_mem as i64, span),
        "free" => Value::filesize(free_mem as i64, span),
        "used" => Value::filesize(used_mem as i64, span),
        "available" => Value::filesize(avail_mem as i64, span),
        "swap total" => Value::filesize(total_swap as i64, span),
        "swap free" => Value::filesize(free_swap as i64, span),
        "swap used" => Value::filesize(used_swap as i64, span),
    };

    Value::record(record, span)
}

pub fn host(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_users_list();

    let mut record = Record::new();

    if let Some(name) = sys.name() {
        record.push("name", Value::string(trim_cstyle_null(name), span));
    }
    if let Some(version) = sys.os_version() {
        record.push("os_version", Value::string(trim_cstyle_null(version), span));
    }

    if let Some(long_version) = sys.long_os_version() {
        record.push(
            "long_os_version",
            Value::string(trim_cstyle_null(long_version), span),
        );
    }

    if let Some(version) = sys.kernel_version() {
        record.push(
            "kernel_version",
            Value::string(trim_cstyle_null(version), span),
        );
    }
    if let Some(hostname) = sys.host_name() {
        record.push("hostname", Value::string(trim_cstyle_null(hostname), span));
    }

    record.push(
        "uptime",
        Value::duration(1000000000 * sys.uptime() as i64, span),
    );

    // Creates a new SystemTime from the specified number of whole seconds
    let d = UNIX_EPOCH + Duration::from_secs(sys.boot_time());
    // Create DateTime from SystemTime
    let datetime = DateTime::<Local>::from(d);
    // Convert to local time and then rfc3339
    let timestamp_str = datetime.with_timezone(datetime.offset()).to_rfc3339();

    record.push("boot_time", Value::string(timestamp_str, span));

    let mut users = vec![];
    for user in sys.users() {
        let mut groups = vec![];
        for group in user.groups() {
            groups.push(Value::String {
                val: trim_cstyle_null(group.to_string()),
                span,
            });
        }

        let record = record! {
            "name" => Value::string(trim_cstyle_null(user.name().to_string()), span),
            "groups" => Value::list(groups, span),
        };

        users.push(Value::record(record, span));
    }

    if !users.is_empty() {
        record.push("sessions", Value::list(users, span));
    }

    Value::record(record, span)
}

pub fn temp(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_components();
    sys.refresh_components_list();

    let mut output = vec![];

    for component in sys.components() {
        let mut record = record! {
            "unit" => Value::string(component.label(), span),
            "temp" => Value::float(component.temperature() as f64, span),
            "high" => Value::float(component.max() as f64, span),
        };

        if let Some(critical) = component.critical() {
            record.push("critical", Value::float(critical as f64, span));
        }
        output.push(Value::record(record, span));
    }

    Value::List { vals: output, span }
}
