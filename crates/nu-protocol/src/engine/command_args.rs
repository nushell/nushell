use super::{EvaluationContext, UnevaluatedCallInfo};

pub struct CommandArgs {
    pub context: EvaluationContext,
    pub call_info: UnevaluatedCallInfo,
    pub input: crate::Value,
}
