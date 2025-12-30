//! `ebpf list` command - list active eBPF probes

use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct EbpfList;

impl Command for EbpfList {
    fn name(&self) -> &str {
        "ebpf list"
    }

    fn description(&self) -> &str {
        "List all active eBPF probes."
    }

    fn signature(&self) -> Signature {
        Signature::build("ebpf list")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "ebpf list",
            description: "List all active eBPF probes",
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
        #[cfg(not(target_os = "linux"))]
        {
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
            run_list(call)
        }
    }
}

#[cfg(target_os = "linux")]
fn run_list(call: &Call) -> Result<PipelineData, ShellError> {
    use crate::loader::get_state;

    let state = get_state();
    let probes = state.list();

    let rows: Vec<Value> = probes
        .into_iter()
        .map(|p| {
            Value::record(
                record! {
                    "id" => Value::int(p.id as i64, call.head),
                    "probe" => Value::string(p.probe_spec, call.head),
                    "uptime" => Value::string(format!("{}s", p.uptime_secs), call.head),
                },
                call.head,
            )
        })
        .collect();

    Ok(Value::list(rows, call.head).into_pipeline_data())
}
