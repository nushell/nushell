#[cfg(target_os = "macos")]
use chrono::{Local, TimeZone};
#[cfg(windows)]
use itertools::Itertools;
use nu_engine::command_prelude::*;

#[cfg(target_os = "linux")]
use procfs::WithCurrentSystemInfo;
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
                "list all available columns for each entry",
                Some('l'),
            )
            .filter()
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "View information about system processes."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["procedures", "operations", "tasks", "ops"]
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
                example: "ps | sort-by mem | last 5",
                result: None,
            },
            Example {
                description: "List the top 3 system processes with the highest CPU usage",
                example: "ps | sort-by cpu | last 3",
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
    let mut output = vec![];
    let span = call.head;
    let long = call.has_flag(engine_state, stack, "long")?;

    for proc in nu_system::collect_proc(Duration::from_millis(100), false) {
        let mut record = Record::new();

        record.push("pid", Value::int(proc.pid() as i64, span));
        record.push("ppid", Value::int(proc.ppid() as i64, span));
        record.push("name", Value::string(proc.name(), span));

        #[cfg(not(windows))]
        {
            // Hide status on Windows until we can find a good way to support it
            record.push("status", Value::string(proc.status(), span));
        }

        record.push("cpu", Value::float(proc.cpu_usage(), span));
        record.push("mem", Value::filesize(proc.mem_size() as i64, span));
        record.push("virtual", Value::filesize(proc.virtual_size() as i64, span));

        if long {
            record.push("command", Value::string(proc.command(), span));
            #[cfg(target_os = "linux")]
            {
                let proc_stat = proc
                    .curr_proc
                    .stat()
                    .map_err(|e| ShellError::GenericError {
                        error: "Error getting process stat".into(),
                        msg: e.to_string(),
                        span: Some(Span::unknown()),
                        help: None,
                        inner: vec![],
                    })?;
                record.push(
                    "start_time",
                    match proc_stat.starttime().get() {
                        Ok(ts) => Value::date(ts.into(), span),
                        Err(_) => Value::nothing(span),
                    },
                );
                record.push("user_id", Value::int(proc.curr_proc.owner() as i64, span));
                // These work and may be helpful, but it just seemed crowded
                // record.push("group_id", Value::int(proc_stat.pgrp as i64, span));
                // record.push("session_id", Value::int(proc_stat.session as i64, span));
                // This may be helpful for ctrl+z type of checking, once we get there
                // record.push("tpg_id", Value::int(proc_stat.tpgid as i64, span));
                record.push("priority", Value::int(proc_stat.priority, span));
                record.push("process_threads", Value::int(proc_stat.num_threads, span));
                record.push("cwd", Value::string(proc.cwd(), span));
            }
            #[cfg(windows)]
            {
                //TODO: There's still more information we can cram in there if we want to
                // see the ProcessInfo struct for more information
                record.push(
                    "start_time",
                    Value::date(proc.start_time.fixed_offset(), span),
                );
                record.push(
                    "user",
                    Value::string(
                        proc.user.clone().name.unwrap_or("unknown".to_string()),
                        span,
                    ),
                );
                record.push(
                    "user_sid",
                    Value::string(
                        proc.user
                            .clone()
                            .sid
                            .iter()
                            .map(|r| r.to_string())
                            .join("-"),
                        span,
                    ),
                );
                record.push("priority", Value::int(proc.priority as i64, span));
                record.push("cwd", Value::string(proc.cwd(), span));
                record.push(
                    "environment",
                    Value::list(
                        proc.environ()
                            .iter()
                            .map(|x| Value::string(x.to_string(), span))
                            .collect(),
                        span,
                    ),
                );
            }
            #[cfg(target_os = "macos")]
            {
                let timestamp = Local
                    .timestamp_nanos(proc.start_time * 1_000_000_000)
                    .into();
                record.push("start_time", Value::date(timestamp, span));
                record.push("user_id", Value::int(proc.user_id, span));
                record.push("priority", Value::int(proc.priority, span));
                record.push("process_threads", Value::int(proc.task_thread_num, span));
                record.push("cwd", Value::string(proc.cwd(), span));
            }
        }

        output.push(Value::record(record, span));
    }

    Ok(output.into_pipeline_data(span, engine_state.signals().clone()))
}
