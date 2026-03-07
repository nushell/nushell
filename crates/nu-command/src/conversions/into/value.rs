use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IntoValue;

impl Command for IntoValue {
    fn name(&self) -> &str {
        "into value"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .description(self.description())
            .extra_description(self.extra_description())
            .input_output_type(Type::Any, Type::Any)
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert custom values into base values."
    }

    fn extra_description(&self) -> &str {
        "Custom values from plugins have a base value representation. \
        This extracts that base value representation. \
        For streams use `collect`."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["custom", "base", "convert", "conversion"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if let PipelineData::Value(v @ Value::Custom { .. }, metadata) = input {
            let span = v.span();
            let val = v.into_custom_value()?;
            return Ok(PipelineData::value(val.to_base_value(span)?, metadata));
        }

        Ok(input)
    }
}
