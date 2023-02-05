use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, PipelineMetadata, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SetMetadata;

impl Command for SetMetadata {
    fn name(&self) -> &str {
        "set-metadata"
    }

    fn usage(&self) -> &str {
        "Assigns the metadata from the metadata argument into the stream"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("set-metadata")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .allow_variants_without_examples(true)
            .required(
                "metadata_var_name",
                SyntaxShape::Record,
                "metadata variable name",
            )
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let val: Value = call.req(engine_state, stack, 0)?;
        let metadata = get_source(val);
        Ok(input.set_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Set the metadata of a variable",
            example: "let-with-metadata a md = ls; $a | set-metadata $md",
            result: None,
        }]
    }
}

pub fn get_source(arg: Value) -> Option<PipelineMetadata> {
    let source_val = match arg {
        Value::Record { cols, vals, .. } => cols
            .iter()
            .zip(vals)
            .find_map(|(col, val)| (col == "source").then_some(val)),
        _ => return None,
    }?;

    source_val.as_string().ok()?.parse().ok()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SetMetadata {})
    }
}
