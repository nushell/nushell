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

/// Get thread group ID / process ID (maps to bpf_get_current_pid_tgid >> 32 in eBPF)
///
/// In Linux, bpf_get_current_pid_tgid() returns:
/// - Lower 32 bits: PID (thread ID in kernel terminology)
/// - Upper 32 bits: TGID (thread group ID = the "PID" that users expect)
///
/// This command returns the TGID, which matches getpid() from userspace.
#[derive(Clone)]
pub struct BpfTgid;

impl Command for BpfTgid {
    fn name(&self) -> &str {
        "bpf-tgid"
    }

    fn description(&self) -> &str {
        "Get the thread group ID (process ID). In eBPF, returns bpf_get_current_pid_tgid() >> 32."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-tgid")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-tgid",
            description: "Get the thread group ID (process ID)",
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
        // At regular runtime, return the actual PID (which is the TGID for the main thread)
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

/// Get current process name/comm (maps to bpf_get_current_comm in eBPF)
///
/// Returns the first 8 bytes of the process name as an i64.
/// This allows simple comparison and emission via bpf-emit.
#[derive(Clone)]
pub struct BpfComm;

impl Command for BpfComm {
    fn name(&self) -> &str {
        "bpf-comm"
    }

    fn description(&self) -> &str {
        "Get the current process name (comm). Returns first 8 bytes as int. In eBPF, maps to bpf_get_current_comm()."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-comm")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-comm",
            description: "Get the first 8 bytes of the current process name as an integer",
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
        // At regular runtime, get the actual process name
        #[cfg(unix)]
        let comm = {
            // Read from /proc/self/comm
            std::fs::read_to_string("/proc/self/comm")
                .unwrap_or_else(|_| "unknown\n".to_string())
                .trim()
                .to_string()
        };
        #[cfg(not(unix))]
        let comm = "unknown".to_string();

        // Convert first 8 bytes to i64
        let mut bytes = [0u8; 8];
        let comm_bytes = comm.as_bytes();
        let len = comm_bytes.len().min(8);
        bytes[..len].copy_from_slice(&comm_bytes[..len]);
        let value = i64::from_le_bytes(bytes);

        Ok(Value::int(value, call.head).into_pipeline_data())
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

    fn extra_description(&self) -> &str {
        r#"Supports both single values (integers) and structured records.
When given a record, all fields are emitted as a single structured event."#
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-emit")
            .input_output_types(vec![
                (Type::Int, Type::Int),
                (Type::Any, Type::Any),  // Support records
            ])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "bpf-pid | bpf-emit",
                description: "Emit the current PID to the perf buffer",
                result: None,
            },
            Example {
                example: "{ pid: (bpf-tgid), time: (bpf-ktime) } | bpf-emit",
                description: "Emit a structured event with PID and timestamp",
                result: None,
            },
        ]
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

/// Emit the current process name to the perf buffer (full 16 bytes)
///
/// Unlike bpf-comm which returns a truncated i64, this emits the full
/// TASK_COMM_LEN (16 bytes) process name as a string event.
#[derive(Clone)]
pub struct BpfEmitComm;

impl Command for BpfEmitComm {
    fn name(&self) -> &str {
        "bpf-emit-comm"
    }

    fn description(&self) -> &str {
        "Emit the current process name to the perf buffer as a string."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-emit-comm")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-emit-comm",
            description: "Emit the current process name",
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
        // At regular runtime, get and print the process name
        #[cfg(unix)]
        let comm = std::fs::read_to_string("/proc/self/comm")
            .unwrap_or_else(|_| "unknown\n".to_string())
            .trim()
            .to_string();
        #[cfg(not(unix))]
        let comm = "unknown".to_string();

        eprintln!("[bpf-emit-comm] {}", comm);
        Ok(Value::string(comm, call.head).into_pipeline_data())
    }
}

/// Read a function argument from the probe context
///
/// In kprobes, reads from pt_regs to get the nth function argument.
/// Architecture-specific: on x86_64, args are in rdi, rsi, rdx, rcx, r8, r9.
#[derive(Clone)]
pub struct BpfArg;

impl Command for BpfArg {
    fn name(&self) -> &str {
        "bpf-arg"
    }

    fn description(&self) -> &str {
        "Read a function argument from the probe context (0-5 on x86_64)."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-arg")
            .required("index", SyntaxShape::Int, "Argument index (0-5)")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "bpf-arg 0",
                description: "Get the first function argument",
                result: None,
            },
            Example {
                example: "bpf-arg 1 | bpf-emit",
                description: "Emit the second function argument",
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
        let index: i64 = call.req(engine_state, stack, 0)?;
        // At regular runtime, we can't access function arguments
        // Return a placeholder value indicating which arg would be read
        eprintln!("[bpf-arg] Would read argument {}", index);
        Ok(Value::int(0, call.head).into_pipeline_data())
    }
}

/// Read the return value from a kretprobe context
///
/// Only valid in kretprobe handlers. Returns the function's return value.
#[derive(Clone)]
pub struct BpfRetval;

impl Command for BpfRetval {
    fn name(&self) -> &str {
        "bpf-retval"
    }

    fn description(&self) -> &str {
        "Read the function return value (only valid in kretprobe)."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-retval")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-retval | bpf-emit",
            description: "Emit the function's return value",
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
        // At regular runtime, we can't access return values
        eprintln!("[bpf-retval] Would read return value");
        Ok(Value::int(0, call.head).into_pipeline_data())
    }
}

/// Read a string from kernel memory and emit it
///
/// Takes a pointer (typically from bpf-arg) and reads a null-terminated
/// string from kernel memory, then emits it to the perf buffer.
#[derive(Clone)]
pub struct BpfReadStr;

/// Read a string from user-space memory and emit it
///
/// Takes a pointer (typically from bpf-arg) and reads a null-terminated
/// string from user-space memory, then emits it to the perf buffer.
/// Use this for syscall arguments that are user-space pointers (like filenames).
#[derive(Clone)]
pub struct BpfReadUserStr;

impl Command for BpfReadStr {
    fn name(&self) -> &str {
        "bpf-read-str"
    }

    fn description(&self) -> &str {
        "Read a string from kernel memory pointer and emit it (max 128 bytes)."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-read-str")
            .input_output_types(vec![(Type::Int, Type::String)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "bpf-arg 1 | bpf-read-str",
                description: "Read filename from second argument (e.g., in openat)",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // At regular runtime, we can't read kernel memory
        let ptr = input.into_value(call.head)?;
        eprintln!("[bpf-read-str] Would read string from pointer {:?}", ptr);
        Ok(Value::string("<kernel string>", call.head).into_pipeline_data())
    }
}

impl Command for BpfReadUserStr {
    fn name(&self) -> &str {
        "bpf-read-user-str"
    }

    fn description(&self) -> &str {
        "Read a string from user-space memory pointer and emit it (max 128 bytes)."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-read-user-str")
            .input_output_types(vec![(Type::Int, Type::String)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "bpf-arg 1 | bpf-read-user-str",
                description: "Read filename from user-space pointer (e.g., in openat)",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // At regular runtime, we can't read user memory
        let ptr = input.into_value(call.head)?;
        eprintln!("[bpf-read-user-str] Would read string from pointer {:?}", ptr);
        Ok(Value::string("<user string>", call.head).into_pipeline_data())
    }
}

/// Filter by process ID - only proceed if current TGID matches
///
/// In eBPF, this checks the current TGID and exits early if not matching.
/// Must be the first command in the pipeline.
#[derive(Clone)]
pub struct BpfFilterPid;

impl Command for BpfFilterPid {
    fn name(&self) -> &str {
        "bpf-filter-pid"
    }

    fn description(&self) -> &str {
        "Only proceed if current process ID matches. Must be first in pipeline."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-filter-pid")
            .required("pid", SyntaxShape::Int, "Process ID to match")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-filter-pid 1234; bpf-tgid | bpf-emit",
            description: "Only emit events for PID 1234",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // At regular runtime, check if we match
        let target_pid: i64 = call.req(engine_state, stack, 0)?;
        let current_pid = std::process::id() as i64;
        if current_pid != target_pid {
            // In regular runtime, we can't "exit early" like eBPF,
            // so we just return an error to indicate filtering
            return Err(ShellError::GenericError {
                error: "Filter not matched".into(),
                msg: format!("Current PID {} does not match filter {}", current_pid, target_pid),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        }
        Ok(PipelineData::Empty)
    }
}

/// Filter by process name - only proceed if current comm matches
///
/// In eBPF, this checks the first 8 bytes of comm and exits early if not matching.
/// Must be the first command in the pipeline.
#[derive(Clone)]
pub struct BpfFilterComm;

impl Command for BpfFilterComm {
    fn name(&self) -> &str {
        "bpf-filter-comm"
    }

    fn description(&self) -> &str {
        "Only proceed if current process name matches. Must be first in pipeline."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-filter-comm")
            .required("comm", SyntaxShape::String, "Process name to match (first 8 chars)")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "bpf-filter-comm 'nginx'; bpf-tgid | bpf-emit",
            description: "Only emit events for nginx processes",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // At regular runtime, check if we match
        let target_comm: String = call.req(engine_state, stack, 0)?;
        #[cfg(unix)]
        let current_comm = std::fs::read_to_string("/proc/self/comm")
            .unwrap_or_else(|_| "unknown\n".to_string())
            .trim()
            .to_string();
        #[cfg(not(unix))]
        let current_comm = "unknown".to_string();

        // Compare first 8 characters (like eBPF does with i64)
        let target_prefix: String = target_comm.chars().take(8).collect();
        let current_prefix: String = current_comm.chars().take(8).collect();

        if current_prefix != target_prefix {
            return Err(ShellError::GenericError {
                error: "Filter not matched".into(),
                msg: format!("Current comm '{}' does not match filter '{}'", current_comm, target_comm),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        }
        Ok(PipelineData::Empty)
    }
}

/// Count occurrences by key (maps to hash map lookup+update in eBPF)
///
/// This command increments a counter for the input value as key.
/// In eBPF, this creates a hash map and performs an atomic increment.
#[derive(Clone)]
pub struct BpfCount;

impl Command for BpfCount {
    fn name(&self) -> &str {
        "bpf-count"
    }

    fn description(&self) -> &str {
        "Count occurrences by key. In eBPF, updates a hash map counter."
    }

    fn signature(&self) -> Signature {
        Signature::build("bpf-count")
            .input_output_types(vec![(Type::Int, Type::Int)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "bpf-pid | bpf-count",
                description: "Count events per PID",
                result: None,
            },
            Example {
                example: "bpf-comm | bpf-count",
                description: "Count events per process name",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // At regular runtime, just pass through (counting happens in eBPF)
        let value = input.into_value(call.head)?;
        Ok(value.into_pipeline_data())
    }
}
