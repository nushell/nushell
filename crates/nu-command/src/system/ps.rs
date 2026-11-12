use chrono::{DateTime, Local};
use nu_engine::command_prelude::*;

use nu_protocol::PipelineMetadata;
use std::time::Duration;

#[derive(Clone)]
pub struct Ps;

impl Command for Ps {
    fn name(&self) -> &str {
        "ps"
    }

    fn signature(&self) -> Signature {
        Signature::build("ps")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .switch(
                "long",
                "List all available columns for each entry.",
                Some('l'),
            )
            .filter()
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "View information about system processes."
    }

    fn extra_description(&self) -> &str {
        "The columns are the same on every platform. `cpu` is the percent of one CPU core the process used during a short (about 100ms) sample, like `top` shows, and `cpu_time` is the total CPU time it has used since it started. `mem` is resident memory.

With `--long`, `virtual` is the size of the process's virtual address space and `private` is the memory only it uses, the number Activity Monitor and Task Manager show as \"Memory\". `read` and `written` count bytes of storage I/O since the process started; on Windows they count all I/O, including pipes and network. `user_id` is the numeric user id on unix and the SID string (for example `S-1-5-18`) on Windows. Windows has priority classes rather than nice values, so its `nice` is the priority class on the nice scale, the way libuv and Node.js map it, and its `process_group_id` is the console process group.

A column is null when the operating system doesn't make that value available for a process: for example, macOS only shows the CPU, memory, threads, environment and working directory of other users' processes to root (their `command` is the executable's path), and the BSDs count disk I/O in blocks rather than bytes. Null sorts after every number and can't be matched with `=~`, so use `compact` to drop those rows before sorting or matching text."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "procedures",
            "operations",
            "tasks",
            "ops",
            "top",
            "tasklist",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_ps(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "List the system processes",
                example: "ps",
                result: None,
            },
            Example {
                description: "List the top 5 system processes with the highest memory usage",
                example: "ps | compact mem | sort-by mem | last 5",
                result: None,
            },
            Example {
                description: "List the top 3 system processes with the highest CPU usage",
                example: "ps | compact cpu | sort-by cpu | last 3",
                result: None,
            },
            Example {
                description: "List the 3 system processes that have used the most CPU time",
                example: "ps | compact cpu_time | sort-by cpu_time | last 3",
                result: None,
            },
            Example {
                description: "List the system processes with 'nu' in their names",
                example: "ps | where name =~ 'nu'",
                result: None,
            },
            Example {
                description: "Get the parent process id of the current nu process",
                example: "ps | where pid == $nu.pid | get ppid",
                result: None,
            },
        ]
    }
}

fn run_ps(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let long = call.has_flag(engine_state, stack, "long")?;
    let filesize = |bytes: Option<u64>| {
        bytes.map_or(Value::nothing(span), |bytes| {
            Value::filesize(bytes as i64, span)
        })
    };

    let output: Vec<Value> = nu_system::collect_proc(Duration::from_millis(100), long)
        .into_iter()
        .map(|proc| {
            let mut record = Record::new();

            record.push("pid", Value::int(proc.pid() as i64, span));
            record.push("ppid", Value::int(proc.ppid() as i64, span));
            record.push("name", Value::string(proc.name(), span));
            record.push("user", proc.user().into_value(span));
            record.push("status", proc.status().into_value(span));
            record.push("cpu", proc.cpu_usage().into_value(span));
            record.push("cpu_time", proc.cpu_time().into_value(span));
            record.push("mem", filesize(proc.mem_size()));

            if long {
                record.push("virtual", filesize(proc.virtual_size()));
                record.push("private", filesize(proc.private_size()));
                record.push("command", Value::string(proc.command(), span));
                record.push("exe", proc.exe().into_value(span));
                record.push(
                    "start_time",
                    proc.start_time()
                        .map(|time| DateTime::<Local>::from(time).fixed_offset())
                        .into_value(span),
                );
                // A uid on unix, a SID string such as "S-1-5-18" on Windows.
                record.push("user_id", proc.user_id().into_value(span));
                record.push("process_group_id", proc.process_group_id().into_value(span));
                record.push("session_id", proc.session_id().into_value(span));
                record.push("priority", proc.priority().into_value(span));
                record.push("nice", proc.nice().into_value(span));
                record.push("threads", proc.thread_count().into_value(span));
                record.push("read", filesize(proc.disk_read()));
                record.push("written", filesize(proc.disk_written()));
                record.push("cwd", proc.cwd().into_value(span));
                record.push("environment", proc.environ().into_value(span));
            }

            Value::record(record, span)
        })
        .collect();

    Ok(output.into_pipeline_data_with_metadata(
        span,
        engine_state.signals().clone(),
        ps_pipeline_metadata(long, span),
    ))
}

/// Builds `ps` output metadata with table width-priority hints.
fn ps_pipeline_metadata(long: bool, span: Span) -> PipelineMetadata {
    let width_priority_columns: &[&str] = if long {
        &["command", "name"]
    } else {
        &["name"]
    };

    PipelineMetadata::default().with_table_width_priority_columns(span, width_priority_columns)
}
