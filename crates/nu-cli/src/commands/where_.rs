use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    hir::ClassifiedCommand, CallInfo, ReturnSuccess, Scope, Signature, SyntaxShape, UntaggedValue,
    Value,
};

pub struct Where;

impl PerItemCommand for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn signature(&self) -> Signature {
        Signature::build("where").required(
            "condition",
            SyntaxShape::Math,
            "the condition that must match",
        )
    }

    fn usage(&self) -> &str {
        "Filter table to match the condition."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        _raw_args: &RawCommandArgs,
        input: Value,
    ) -> Result<OutputStream, ShellError> {
        let block = call_info.args.expect_nth(0)?.clone();

        let condition = match block {
            Value {
                value: UntaggedValue::Block(block),
                tag,
            } => {
                if block.block.len() != 1 {
                    return Err(ShellError::labeled_error(
                        "Expected a condition",
                        "expected a condition",
                        tag,
                    ));
                }
                match block.block[0].list.get(0) {
                    Some(item) => match item {
                        ClassifiedCommand::Expr(expr) => expr.clone(),
                        _ => {
                            return Err(ShellError::labeled_error(
                                "Expected a condition",
                                "expected a condition",
                                tag,
                            ))
                        }
                    },
                    None => {
                        return Err(ShellError::labeled_error(
                            "Expected a condition",
                            "expected a condition",
                            tag,
                        ));
                    }
                }
            }
            Value { tag, .. } => {
                return Err(ShellError::labeled_error(
                    "Expected a condition",
                    "expected a condition",
                    tag,
                ));
            }
        };

        //FIXME: should we use the scope that's brought in as well?
        let condition = evaluate_baseline_expr(&condition, registry, &Scope::new(input.clone()))?;

        let stream = match condition.as_bool() {
            Ok(b) => {
                if b {
                    VecDeque::from(vec![Ok(ReturnSuccess::Value(input))])
                } else {
                    VecDeque::new()
                }
            }
            Err(e) => return Err(e),
        };

        Ok(stream.into())
    }
}
