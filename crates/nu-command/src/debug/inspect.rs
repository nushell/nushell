use super::inspect_table;
use nu_engine::command_prelude::*;
use nu_utils::terminal_size;

#[derive(Clone)]
pub struct Inspect;

impl Command for Inspect {
    fn name(&self) -> &str {
        "inspect"
    }

    fn description(&self) -> &str {
        "Inspect pipeline results while running a pipeline."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("inspect")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .allow_variants_without_examples(true)
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let input_metadata = input.metadata();
        let input_val = input.into_value(call.head)?;
        if input_val.is_nothing() {
            return Err(ShellError::PipelineEmpty {
                dst_span: call.head,
            });
        }
        let original_input = input_val.clone();
        let description = input_val.get_type().to_string();

        let (cols, _rows) = terminal_size().unwrap_or((0, 0));

        let table = inspect_table::build_table(engine_state, input_val, description, cols as usize);

        // Note that this is printed to stderr. The reason for this is so it doesn't disrupt the regular nushell
        // tabular output. If we printed to stdout, nushell would get confused with two outputs.
        eprintln!("{table}\n");

        Ok(original_input.into_pipeline_data_with_metadata(input_metadata))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Inspect pipeline results",
            example: "ls | inspect | get name | inspect",
            result: None,
        }]
    }
}
