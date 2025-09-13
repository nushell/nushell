use nu_engine::command_prelude::*;
use sysinfo::System;

#[derive(Clone)]
pub struct SysMem;

impl Command for SysMem {
    fn name(&self) -> &str {
        "sys mem"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys mem")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::record())])
    }

    fn description(&self) -> &str {
        "View information about the system memory."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(mem(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Show info about the system memory",
            example: "sys mem",
            result: None,
        }]
    }
}

fn mem(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_memory();

    let record = record! {
        "total" => Value::filesize(sys.total_memory() as i64, span),
        "free" => Value::filesize(sys.free_memory() as i64, span),
        "used" => Value::filesize(sys.used_memory() as i64, span),
        "available" => Value::filesize(sys.available_memory() as i64, span),
        "swap total" => Value::filesize(sys.total_swap() as i64, span),
        "swap free" => Value::filesize(sys.free_swap() as i64, span),
        "swap used" => Value::filesize(sys.used_swap() as i64, span),
    };

    Value::record(record, span)
}
