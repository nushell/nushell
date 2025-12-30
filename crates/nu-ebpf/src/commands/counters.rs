//! Display counter values from bpf-count

use nu_engine::command_prelude::*;

use crate::loader::get_state;

/// Display counter values from an attached probe
#[derive(Clone)]
pub struct EbpfCounters;

impl Command for EbpfCounters {
    fn name(&self) -> &str {
        "ebpf counters"
    }

    fn description(&self) -> &str {
        "Display counter values from a probe using bpf-count"
    }

    fn signature(&self) -> Signature {
        Signature::build("ebpf counters")
            .required("id", SyntaxShape::Int, "Probe ID to get counters from")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "ebpf counters 1",
            description: "Show counter values from probe 1",
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
        let id: i64 = call.req(engine_state, stack, 0)?;
        let span = call.head;

        let state = get_state();
        let entries = state.get_counters(id as u32).map_err(|e| ShellError::GenericError {
            error: "Failed to get counters".into(),
            msg: e.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;

        // Convert entries to a table
        let records: Vec<Value> = entries
            .into_iter()
            .map(|entry| {
                Value::record(
                    record! {
                        "key" => Value::int(entry.key, span),
                        "count" => Value::int(entry.count, span),
                    },
                    span,
                )
            })
            .collect();

        Ok(Value::list(records, span).into_pipeline_data())
    }
}
