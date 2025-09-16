use nu_engine::command_prelude::*;
use nu_protocol::{DeprecationEntry, DeprecationType, ReportMode, report_shell_warning};

use crate::DetectTypes;

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
            // TODO: remove these after deprecation phase
            .named(
                "columns",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "list of columns to update",
                Some('c'),
            )
            .switch(
                "prefer-filesizes",
                "For ints display them as human-readable file sizes",
                Some('f'),
            )
            .allow_variants_without_examples(true)
    }

    fn deprecation_info(&self) -> Vec<DeprecationEntry> {
        vec![
            DeprecationEntry {
                ty: DeprecationType::Flag("columns".to_string()),
                report_mode: ReportMode::EveryUse,
                since: Some("0.108.0".into()),
                expected_removal: Some("0.109.0".into()),
                help: Some("Use this flag on `detect types`.".into()),
            },
            DeprecationEntry {
                ty: DeprecationType::Flag("prefer-filesizes".to_string()),
                report_mode: ReportMode::EveryUse,
                since: Some("0.108.0".into()),
                expected_removal: Some("0.109.0".into()),
                help: Some("Use this flag on `detect types`.".into()),
            },
        ]
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if let PipelineData::Value(Value::Custom { val, internal_span }, metadata) = input {
            return Ok(PipelineData::value(
                val.to_base_value(internal_span)?,
                metadata,
            ));
        }

        report_shell_warning(
            engine_state,
            &ShellWarning::Deprecated {
                dep_type: "Moved Command".into(),
                label: "Detecting types of tables is moved to `detect types`.".into(),
                span: call.head,
                help: Some("Use `detect types` instead. In the future this will be a no-op for nushell native values.".into()),
                report_mode: ReportMode::EveryUse,
            },
        );
        DetectTypes.run(engine_state, stack, call, input)

        // After deprecation period, this rest will be a noop.
        // Ok(input)
    }
}
