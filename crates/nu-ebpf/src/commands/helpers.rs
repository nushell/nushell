//! BPF helper commands
//!
//! These commands map to BPF helper functions when compiled to eBPF.
//! At regular runtime, they provide equivalent functionality.

use nu_engine::command_prelude::*;

/// Get current process ID (maps to bpf_get_current_pid_tgid in eBPF)
#[derive(Clone)]
pub struct BpfPid;

impl Command for BpfPid {
    fn name(&self) -> &str {
        "bpf-pid"
    }

    fn description(&self) -> &str {
        "Get the current process ID. In eBPF, maps to bpf_get_current_pid_tgid()."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-pid")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-pid",
            description: "Get the current PID",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // At regular runtime, return the actual PID
        let pid = std::process::id() as i64;
        Ok(Value::int(pid, call.head).into_pipeline_data())
    }
}

/// Get current user ID (maps to bpf_get_current_uid_gid in eBPF)
#[derive(Clone)]
pub struct BpfUid;

impl Command for BpfUid {
    fn name(&self) -> &str {
        "bpf-uid"
    }

    fn description(&self) -> &str {
        "Get the current user ID. In eBPF, maps to bpf_get_current_uid_gid()."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-uid")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-uid",
            description: "Get the current UID",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // At regular runtime, return the actual UID
        #[cfg(unix)]
        let uid = unsafe { libc::getuid() } as i64;
        #[cfg(not(unix))]
        let uid = 0i64;
        Ok(Value::int(uid, call.head).into_pipeline_data())
    }
}

/// Get kernel time in nanoseconds (maps to bpf_ktime_get_ns in eBPF)
#[derive(Clone)]
pub struct BpfKtime;

impl Command for BpfKtime {
    fn name(&self) -> &str {
        "bpf-ktime"
    }

    fn description(&self) -> &str {
        "Get kernel monotonic time in nanoseconds. In eBPF, maps to bpf_ktime_get_ns()."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-ktime")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-ktime",
            description: "Get kernel time in nanoseconds",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // At regular runtime, use system monotonic time
        use std::time::Instant;
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let start = START.get_or_init(Instant::now);
        let elapsed = start.elapsed().as_nanos() as i64;
        Ok(Value::int(elapsed, call.head).into_pipeline_data())
    }
}

/// Emit a value to the perf buffer (maps to bpf_perf_event_output in eBPF)
#[derive(Clone)]
pub struct BpfEmit;

impl Command for BpfEmit {
    fn name(&self) -> &str {
        "bpf-emit"
    }

    fn description(&self) -> &str {
        "Emit a value to the eBPF perf buffer. In eBPF, outputs to userspace via perf events."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-emit")
            .input_output_types(vec![(Type::Int, Type::Int)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-pid | bpf-emit",
            description: "Emit the current PID to the perf buffer",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // At regular runtime, just pass through the value and print it
        // (in eBPF, this would output to the perf buffer)
        let value = input.into_value(call.head)?;
        eprintln!("[bpf-emit] {}", value.to_expanded_string(", ", &nu_protocol::Config::default()));
        Ok(value.into_pipeline_data())
    }
}
