use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use reedline::HistoryItemId;

use super::utils::history_path;
use super::utils::reedline_history;

#[derive(Clone)]
pub struct HistoryRemoveItem;

impl Command for HistoryRemoveItem {
    fn name(&self) -> &str {
        "history remove-item"
    }

    fn usage(&self) -> &str {
        "Remove an item from history."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history remove-item")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("id", SyntaxShape::Int, "ID of the history item to delete")
            .category(Category::History)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let Some(history) = engine_state.history_config() else {
            return Ok(PipelineData::empty());
        };

        let Some(config_path) = nu_path::config_dir() else {
            return Err(ShellError::FileNotFound { span: call.head });
        };
        let history_path = history_path(config_path, history);
        let Some(mut history_reader) = reedline_history(history, history_path) else {
            return Err(ShellError::FileNotFound { span: call.head });
        };
        let id: Spanned<usize> = call.req(engine_state, stack, 0)?;

        history_reader
            .delete(HistoryItemId::new(id.item as i64))
            .map_err(|e| ShellError::IncorrectValue {
                msg: format!("Could not delete history item of id {}: {}", id.item, e),
                val_span: id.span,
                call_span: call.span(),
            })?;
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        todo!();
    }
}
