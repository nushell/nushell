use crate::evaluate::evaluate_args::evaluate_args;
use crate::evaluation_context::EvaluationContext;
use nu_errors::ShellError;
use nu_protocol::hir;
use nu_protocol::CallInfo;
use nu_source::Tag;

#[derive(Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: hir::Call,
    pub name_tag: Tag,
}

impl UnevaluatedCallInfo {
    pub fn evaluate(self, ctx: &EvaluationContext) -> Result<CallInfo, ShellError> {
        let args = evaluate_args(&self.args, ctx)?;

        Ok(CallInfo {
            args,
            name_tag: self.name_tag,
        })
    }

    pub fn switch_present(&self, switch: &str) -> bool {
        self.args.switch_preset(switch)
    }
}
