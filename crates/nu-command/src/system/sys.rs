use chrono::prelude::DateTime;
use chrono::Local;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Value,
};
use std::time::{Duration, UNIX_EPOCH};
use sysinfo::{ComponentExt, CpuExt, DiskExt, NetworkExt, System, SystemExt, UserExt};

#[derive(Clone)]
pub struct Sys;

impl Command for Sys {
    fn name(&self) -> &str {
        "sys"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys").filter().category(Category::System)
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        run_sys(call)
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

fn run_sys(call: &Call) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let mut sys = System::new();

    let mut headers = vec![];
    let mut values = vec![];

    if let Some(value) = host(&mut sys, span) {
        headers.push("host".into());
        values.push(value);
    }
    if let Some(value) = cpu(&mut sys, span) {
        headers.push("cpu".into());
        values.push(value);
    }
    if let Some(value) = disks(&mut sys, span) {
        headers.push("disks".into());
        values.push(value);
    }
    if let Some(value) = mem(&mut sys, span) {
        headers.push("mem".into());
        values.push(value);
    }
    if let Some(value) = temp(&mut sys, span) {
        headers.push("temp".into());
        values.push(value);
    }
    if let Some(value) = net(&mut sys, span) {
        headers.push("net".into());
        values.push(value);
    }

    Ok(Value::Record {
        cols: headers,
        vals: values,
        span,
    }
    .into_pipeline_data())
}

pub fn trim_cstyle_null(s: String) -> String {
    s.trim_matches(char::from(0)).to_string()
}

pub fn disks(sys: &mut System, span: Span) -> Option<Value> {
    sys.refresh_disks();
    sys.refresh_disks_list();

    let mut output = vec![];
    for disk in sys.disks() {
        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("device".into());
        vals.push(Value::String {
            val: trim_cstyle_null(disk.name().to_string_lossy().to_string()),
            span,
        });

        cols.push("type".into());
        vals.push(Value::String {
            val: trim_cstyle_null(String::from_utf8_lossy(disk.file_system()).to_string()),
            span,
        });

        cols.push("mount".into());
        vals.push(Value::String {
            val: disk.mount_point().to_string_lossy().to_string(),
            span,
        });

        cols.push("total".into());
        vals.push(Value::Filesize {
            val: disk.total_space() as i64,
            span,
        });

        cols.push("free".into());
        vals.push(Value::Filesize {
            val: disk.available_space() as i64,
            span,
        });

        cols.push("removable".into());
        vals.push(Value::Bool {
            val: disk.is_removable(),
            span,
        });

        cols.push("removable".into());
        vals.push(Value::String {
            val: format!("{:?}", disk.type_()),
            span,
        });

        output.push(Value::Record { cols, vals, span });
    }
    if !output.is_empty() {
        Some(Value::List { vals: output, span })
    } else {
        None
    }
}

pub fn net(sys: &mut System, span: Span) -> Option<Value> {
    sys.refresh_networks();
    sys.refresh_networks_list();

    let mut output = vec![];
    for (iface, data) in sys.networks() {
        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("name".into());
        vals.push(Value::String {
            val: trim_cstyle_null(iface.to_string()),
            span,
        });

        cols.push("sent".into());
        vals.push(Value::Filesize {
            val: data.total_transmitted() as i64,
            span,
        });

        cols.push("recv".into());
        vals.push(Value::Filesize {
            val: data.total_received() as i64,
            span,
        });

        output.push(Value::Record { cols, vals, span });
    }
    if !output.is_empty() {
        Some(Value::List { vals: output, span })
    } else {
        None
    }
}

pub fn cpu(sys: &mut System, span: Span) -> Option<Value> {
    sys.refresh_cpu();

    let mut output = vec![];
    for cpu in sys.cpus() {
        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("name".into());
        vals.push(Value::String {
            val: trim_cstyle_null(cpu.name().to_string()),
            span,
        });

        cols.push("brand".into());
        vals.push(Value::String {
            val: trim_cstyle_null(cpu.brand().to_string()),
            span,
        });

        cols.push("freq".into());
        vals.push(Value::Int {
            val: cpu.frequency() as i64,
            span,
        });

        cols.push("cpu_usage".into());
        vals.push(Value::Float {
            val: cpu.cpu_usage() as f64,
            span,
        });

        let load_avg = sys.load_average();
        cols.push("load_average".into());
        vals.push(Value::String {
            val: trim_cstyle_null(format!(
                "{:.2}, {:.2}, {:.2}",
                load_avg.one, load_avg.five, load_avg.fifteen
            )),
            span,
        });

        cols.push("vendor_id".into());
        vals.push(Value::String {
            val: trim_cstyle_null(cpu.vendor_id().to_string()),
            span,
        });

        cols.push("freq".into());
        vals.push(Value::Int {
            val: sys.physical_core_count().unwrap_or(0) as i64,
            span,
        });

        output.push(Value::Record { cols, vals, span });
    }

    if !output.is_empty() {
        Some(Value::List { vals: output, span })
    } else {
        None
    }
}

pub fn mem(sys: &mut System, span: Span) -> Option<Value> {
    sys.refresh_memory();

    let mut cols = vec![];
    let mut vals = vec![];

    let total_mem = sys.total_memory();
    let free_mem = sys.free_memory();
    let used_mem = sys.used_memory();
    let avail_mem = sys.available_memory();

    let total_swap = sys.total_swap();
    let free_swap = sys.free_swap();
    let used_swap = sys.used_swap();

    cols.push("total".into());
    vals.push(Value::Filesize {
        val: total_mem as i64,
        span,
    });

    cols.push("free".into());
    vals.push(Value::Filesize {
        val: free_mem as i64,
        span,
    });

    cols.push("used".into());
    vals.push(Value::Filesize {
        val: used_mem as i64,
        span,
    });

    cols.push("available".into());
    vals.push(Value::Filesize {
        val: avail_mem as i64,
        span,
    });

    cols.push("swap total".into());
    vals.push(Value::Filesize {
        val: total_swap as i64,
        span,
    });

    cols.push("swap free".into());
    vals.push(Value::Filesize {
        val: free_swap as i64,
        span,
    });

    cols.push("swap used".into());
    vals.push(Value::Filesize {
        val: used_swap as i64,
        span,
    });

    Some(Value::Record { cols, vals, span })
}

pub fn host(sys: &mut System, span: Span) -> Option<Value> {
    sys.refresh_users_list();

    let mut cols = vec![];
    let mut vals = vec![];

    if let Some(name) = sys.name() {
        cols.push("name".into());
        vals.push(Value::String {
            val: trim_cstyle_null(name),
            span,
        });
    }
    if let Some(version) = sys.os_version() {
        cols.push("os_version".into());
        vals.push(Value::String {
            val: trim_cstyle_null(version),
            span,
        });
    }

    if let Some(long_version) = sys.long_os_version() {
        cols.push("long_os_version".into());
        vals.push(Value::String {
            val: trim_cstyle_null(long_version),
            span,
        });
    }

    if let Some(version) = sys.kernel_version() {
        cols.push("kernel_version".into());
        vals.push(Value::String {
            val: trim_cstyle_null(version),
            span,
        });
    }
    if let Some(hostname) = sys.host_name() {
        cols.push("hostname".into());
        vals.push(Value::String {
            val: trim_cstyle_null(hostname),
            span,
        });
    }

    cols.push("uptime".into());
    vals.push(Value::Duration {
        val: 1000000000 * sys.uptime() as i64,
        span,
    });

    // Creates a new SystemTime from the specified number of whole seconds
    let d = UNIX_EPOCH + Duration::from_secs(sys.boot_time());
    // Create DateTime from SystemTime
    let datetime = DateTime::<Local>::from(d);
    // Convert to local time and then rfc3339
    let timestamp_str = datetime.with_timezone(datetime.offset()).to_rfc3339();

    cols.push("boot_time".into());
    vals.push(Value::String {
        val: timestamp_str,
        span,
    });

    let mut users = vec![];
    for user in sys.users() {
        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("name".into());
        vals.push(Value::String {
            val: trim_cstyle_null(user.name().to_string()),
            span,
        });

        let mut groups = vec![];
        for group in user.groups() {
            groups.push(Value::String {
                val: trim_cstyle_null(group.to_string()),
                span,
            });
        }

        cols.push("groups".into());
        vals.push(Value::List { vals: groups, span });

        users.push(Value::Record { cols, vals, span });
    }

    if !users.is_empty() {
        cols.push("sessions".into());
        vals.push(Value::List { vals: users, span });
    }

    Some(Value::Record { cols, vals, span })
}

pub fn temp(sys: &mut System, span: Span) -> Option<Value> {
    sys.refresh_components();
    sys.refresh_components_list();

    let mut output = vec![];

    for component in sys.components() {
        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("unit".into());
        vals.push(Value::String {
            val: component.label().to_string(),
            span,
        });

        cols.push("temp".into());
        vals.push(Value::Float {
            val: component.temperature() as f64,
            span,
        });

        cols.push("high".into());
        vals.push(Value::Float {
            val: component.max() as f64,
            span,
        });

        if let Some(critical) = component.critical() {
            cols.push("critical".into());
            vals.push(Value::Float {
                val: critical as f64,
                span,
            });
        }
        output.push(Value::Record { cols, vals, span });
    }

    if !output.is_empty() {
        Some(Value::List { vals: output, span })
    } else {
        None
    }
}
