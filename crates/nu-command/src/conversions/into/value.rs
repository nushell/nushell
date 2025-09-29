use std::sync::OnceLock;

use nu_engine::command_prelude::*;
use nu_protocol::{
    BlockId, DeprecationEntry, DeprecationType, ReportMode, debugger::WithoutDebug,
    report_shell_warning,
};

// TODO: remove this after deprecation phase
static DEPRECATED_REDIRECT_BLOCK_ID: OnceLock<BlockId> = OnceLock::new();

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
    }

    fn deprecation_info(&self) -> Vec<DeprecationEntry> {
        vec![
            DeprecationEntry {
                ty: DeprecationType::Flag("columns".to_string()),
                report_mode: ReportMode::EveryUse,
                since: Some("0.108.0".into()),
                expected_removal: Some("0.109.0".into()),
                help: Some("Use this flag on `detect type`.".into()),
            },
            DeprecationEntry {
                ty: DeprecationType::Flag("prefer-filesizes".to_string()),
                report_mode: ReportMode::EveryUse,
                since: Some("0.108.0".into()),
                expected_removal: Some("0.109.0".into()),
                help: Some("Use this flag on `detect type`.".into()),
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

        if let Some(block_id) = DEPRECATED_REDIRECT_BLOCK_ID.get() {
            report_shell_warning(
            engine_state,
            &ShellWarning::Deprecated {
                dep_type: "Moved Command".into(),
                label: "Detecting types of tables is moved to `detect types`.".into(),
                span: call.head,
                help: Some("Use `update cells {detect type}` instead. In the future this will be a no-op for nushell native values.".into()),
                report_mode: ReportMode::EveryUse,
            },
        );

            let Some(block) = engine_state.try_get_block(*block_id) else {
                return Err(ShellError::GenericError {
                    error: "Block ID not found".into(),
                    msg: format!("Block ID {} not found in this EngineState", block_id.get()),
                    span: Some(call.head),
                    help: Some("Make sure the same EngineState for IntoValue::add_deprecated_call was used here.".into()),
                    inner: vec![],
                });
            };
            let execution_data =
                nu_engine::eval_block::<WithoutDebug>(engine_state, stack, block, input)?;
            return Ok(execution_data.body);
        }

        Ok(input)
    }
}

impl IntoValue {
    // TODO: remove this method after deprecation phase
    // This is a major hack to get the `update cell {detect type}` call possible without writing to
    // much code that will be thrown away anyway.
    pub fn add_deprecated_call(engine_state: &mut EngineState) {
        let code = b"update cells {detect type}";
        let mut working_set = StateWorkingSet::new(engine_state);
        let block = nu_parser::parse(
            &mut working_set,
            Some("`into value` inner redirect"),
            code,
            false,
        );
        debug_assert!(
            working_set.parse_errors.is_empty(),
            "parsing `update cells {{detect type}}` errored"
        );
        debug_assert!(
            working_set.compile_errors.is_empty(),
            "compiling `update cells {{detect type}}` errored"
        );
        let block_id = working_set.add_block(block);
        if engine_state.merge_delta(working_set.delta).is_err() {
            log::error!("could not merge delta for deprecated redirect block of `into value`");
            return;
        }

        if DEPRECATED_REDIRECT_BLOCK_ID.set(block_id).is_err() {
            log::error!("could not set block id for deprecated redirect block of `into value`");
        }
    }
}
