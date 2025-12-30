//! `ebpf attach` command - attach an eBPF probe

use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct EbpfAttach;

impl Command for EbpfAttach {
    fn name(&self) -> &str {
        "ebpf attach"
    }

    fn description(&self) -> &str {
        "Attach an eBPF probe to a kernel function or tracepoint."
    }

    fn extra_description(&self) -> &str {
        r#"This command compiles a Nushell closure to eBPF bytecode and attaches
it to the specified kernel probe point. The closure runs in the kernel
whenever the probe point is hit.

Note: Currently only a minimal "hello world" program is generated.
The closure argument is accepted but not yet compiled.

Probe specification format:
  kprobe:function_name     - Attach to function entry
  kretprobe:function_name  - Attach to function return
  tracepoint:category/name - Attach to a tracepoint

Requirements:
  - Linux kernel 4.18+
  - CAP_BPF capability or root access"#
    }

    fn signature(&self) -> Signature {
        Signature::build("ebpf attach")
            .input_output_types(vec![
                (Type::Nothing, Type::Int),
                (Type::Nothing, Type::Binary),
            ])
            .required(
                "probe",
                SyntaxShape::String,
                "The probe point (e.g., 'kprobe:sys_clone').",
            )
            .required(
                "closure",
                SyntaxShape::Closure(None),
                "The closure to compile to eBPF (currently ignored - generates hello world).",
            )
            .switch(
                "dry-run",
                "Generate bytecode but don't load into kernel",
                Some('n'),
            )
            .category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["bpf", "kernel", "trace", "probe", "kprobe", "tracepoint"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "ebpf attach 'kprobe:sys_clone' {|| 0 }",
                description: "Attach a probe to the sys_clone syscall",
                result: None,
            },
            Example {
                example: "let id = ebpf attach 'kprobe:sys_read' {|| 0 }; ebpf detach $id",
                description: "Attach and then detach a probe",
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
            run_attach(engine_state, stack, call)
        }
    }
}

#[cfg(target_os = "linux")]
fn run_attach(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    use crate::compiler::{EbpfProgram, IrToEbpfCompiler};
    use crate::loader::{get_state, parse_probe_spec, LoadError};

    let probe_spec: String = call.req(engine_state, stack, 0)?;
    let closure: Closure = call.req(engine_state, stack, 1)?;
    let dry_run = call.has_flag(engine_state, stack, "dry-run")?;

    // Parse the probe specification
    let (prog_type, target) = parse_probe_spec(&probe_spec).map_err(|e| ShellError::GenericError {
        error: "Invalid probe specification".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: Some("Use format like 'kprobe:sys_clone' or 'tracepoint:syscalls/sys_enter_read'".into()),
        inner: vec![],
    })?;

    // Get the block for this closure
    let block = engine_state.get_block(closure.block_id);

    // Try to compile the closure's IR to eBPF
    let compile_result = if let Some(ir_block) = &block.ir_block {
        match IrToEbpfCompiler::compile_full(ir_block, engine_state) {
            Ok(result) => result,
            Err(e) => {
                // Fall back to hello world if compilation fails
                eprintln!("Warning: IR compilation failed ({e}), using fallback");
                use crate::compiler::{CompileResult, EbpfInsn, EbpfReg};
                let mut builder = crate::compiler::instruction::EbpfBuilder::new();
                builder
                    .push(EbpfInsn::mov64_imm(EbpfReg::R0, 0))
                    .push(EbpfInsn::exit());
                CompileResult {
                    bytecode: builder.build(),
                    maps: Vec::new(),
                    relocations: Vec::new(),
                    event_schema: None,
                }
            }
        }
    } else {
        // No IR available, use fallback
        use crate::compiler::{CompileResult, EbpfInsn, EbpfReg};
        let mut builder = crate::compiler::instruction::EbpfBuilder::new();
        builder
            .push(EbpfInsn::mov64_imm(EbpfReg::R0, 0))
            .push(EbpfInsn::exit());
        CompileResult {
            bytecode: builder.build(),
            maps: Vec::new(),
            relocations: Vec::new(),
            event_schema: None,
        }
    };

    let program = EbpfProgram::with_maps(
        prog_type,
        &target,
        "nushell_ebpf",
        compile_result.bytecode,
        compile_result.maps,
        compile_result.relocations,
        compile_result.event_schema,
    );

    if dry_run {
        // Return the ELF bytes for inspection
        let elf = program.to_elf().map_err(|e| ShellError::GenericError {
            error: "Failed to generate ELF".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

        return Ok(Value::binary(elf, call.head).into_pipeline_data());
    }

    // Load and attach the program
    let state = get_state();
    let probe_id = state.attach(&program).map_err(|e| {
        let (error, help) = match &e {
            LoadError::PermissionDenied => (
                "Permission denied".into(),
                Some("Try running with sudo or grant CAP_BPF capability".into()),
            ),
            _ => (e.to_string(), None),
        };
        ShellError::GenericError {
            error: "Failed to attach eBPF probe".into(),
            msg: error,
            span: Some(call.head),
            help,
            inner: vec![],
        }
    })?;

    Ok(Value::int(probe_id as i64, call.head).into_pipeline_data())
}
