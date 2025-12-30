//! `ebpf trace` command - read kernel trace output

use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct EbpfTrace;

impl Command for EbpfTrace {
    fn name(&self) -> &str {
        "ebpf trace"
    }

    fn description(&self) -> &str {
        "Read eBPF trace output from the kernel."
    }

    fn extra_description(&self) -> &str {
        r#"This command reads from /sys/kernel/debug/tracing/trace_pipe, which
receives output from bpf_trace_printk() calls in eBPF programs.

The output is streamed as records with fields:
  - task: The task name
  - pid: The process ID
  - cpu: The CPU number
  - flags: Trace flags
  - timestamp: The kernel timestamp
  - message: The trace message

Press Ctrl+C to stop reading.

Note: Requires debugfs mounted and appropriate permissions."#
    }

    fn signature(&self) -> Signature {
        Signature::build("ebpf trace")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .switch("raw", "Output raw lines instead of parsing", Some('r'))
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "ebpf trace | first 10",
                description: "Read 10 trace events",
                result: None,
            },
            Example {
                example: "ebpf trace --raw | lines | first 5",
                description: "Read 5 raw trace lines",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (engine_state, stack, call);
            return Err(ShellError::GenericError {
                error: "eBPF is only supported on Linux".into(),
                msg: "This command requires a Linux system with eBPF support".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        }

        #[cfg(target_os = "linux")]
        {
            run_trace(engine_state, stack, call)
        }
    }
}

#[cfg(target_os = "linux")]
fn run_trace(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let raw = call.has_flag(engine_state, stack, "raw")?;
    let span = call.head;

    // Open trace_pipe
    let trace_pipe_path = "/sys/kernel/debug/tracing/trace_pipe";
    let file = File::open(trace_pipe_path).map_err(|e| {
        let help = if e.kind() == std::io::ErrorKind::PermissionDenied {
            Some("Try running with sudo or ensure debugfs is mounted".into())
        } else if e.kind() == std::io::ErrorKind::NotFound {
            Some("Ensure debugfs is mounted: mount -t debugfs none /sys/kernel/debug".into())
        } else {
            None
        };
        ShellError::GenericError {
            error: "Failed to open trace_pipe".into(),
            msg: e.to_string(),
            span: Some(span),
            help,
            inner: vec![],
        }
    })?;

    let reader = BufReader::new(file);
    let signals = engine_state.signals().clone();
    let signals_for_iter = signals.clone();

    // Create an iterator that reads lines and converts to Values
    let iter = reader.lines().filter_map(move |line_result| {
        // Check for interrupt
        if signals_for_iter.interrupted() {
            return None;
        }

        match line_result {
            Ok(line) => {
                if line.trim().is_empty() {
                    return None;
                }
                if raw {
                    Some(Value::string(line, span))
                } else {
                    Some(parse_trace_line(&line, span))
                }
            }
            Err(_) => None,
        }
    });

    Ok(iter.into_pipeline_data(span, signals))
}

/// Parse a trace_pipe line into a record
///
/// Format: "            task-PID     [CPU] FLAGS  TIMESTAMP: message"
/// Example: "           <...>-12345  [001] d...  12345.678901: bpf_trace_printk: Hello"
#[cfg(target_os = "linux")]
fn parse_trace_line(line: &str, span: Span) -> Value {
    // Try to parse the structured format
    // If parsing fails, return a simple record with just the raw line

    let parts: Vec<&str> = line.splitn(2, ": ").collect();

    if parts.len() == 2 {
        // Try to parse the header
        let header = parts[0];
        let message = parts[1];

        // Parse header: "task-PID     [CPU] FLAGS  TIMESTAMP"
        if let Some(parsed) = parse_trace_header(header) {
            return Value::record(
                record! {
                    "task" => Value::string(parsed.task, span),
                    "pid" => Value::int(parsed.pid, span),
                    "cpu" => Value::int(parsed.cpu, span),
                    "flags" => Value::string(parsed.flags, span),
                    "timestamp" => Value::float(parsed.timestamp, span),
                    "message" => Value::string(message.to_string(), span),
                },
                span,
            );
        }
    }

    // Fallback: return raw line
    Value::record(
        record! {
            "raw" => Value::string(line.to_string(), span),
        },
        span,
    )
}

#[cfg(target_os = "linux")]
struct TraceHeader {
    task: String,
    pid: i64,
    cpu: i64,
    flags: String,
    timestamp: f64,
}

#[cfg(target_os = "linux")]
fn parse_trace_header(header: &str) -> Option<TraceHeader> {
    // Format: "            task-PID     [CPU] FLAGS  TIMESTAMP"
    // The task-PID part is right-aligned in a 16-char field

    let header = header.trim();

    // Find the last '-' before '[' to split task-pid
    let bracket_pos = header.find('[')?;
    let task_pid_part = header[..bracket_pos].trim();
    let rest = &header[bracket_pos..];

    // Split task-pid on the last '-'
    let last_dash = task_pid_part.rfind('-')?;
    let task = task_pid_part[..last_dash].trim().to_string();
    let pid: i64 = task_pid_part[last_dash + 1..].trim().parse().ok()?;

    // Parse [CPU]
    let close_bracket = rest.find(']')?;
    let cpu: i64 = rest[1..close_bracket].trim().parse().ok()?;

    let after_cpu = rest[close_bracket + 1..].trim();

    // Split on whitespace to get FLAGS and TIMESTAMP
    let mut parts = after_cpu.split_whitespace();
    let flags = parts.next()?.to_string();
    let timestamp_str = parts.next()?;

    // Remove trailing colon from timestamp if present
    let timestamp_str = timestamp_str.trim_end_matches(':');
    let timestamp: f64 = timestamp_str.parse().ok()?;

    Some(TraceHeader {
        task,
        pid,
        cpu,
        flags,
        timestamp,
    })
}
