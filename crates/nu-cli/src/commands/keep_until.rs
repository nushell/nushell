use crate::commands::WholeStreamCommand;
use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{hir::ClassifiedCommand, Signature, SyntaxShape, UntaggedValue, Value};

pub struct KeepUntil;

impl WholeStreamCommand for KeepUntil {
    fn name(&self) -> &str {
        "keep-until"
    }

    fn signature(&self) -> Signature {
        Signature::build("keep-until")
            .required(
                "condition",
                SyntaxShape::Math,
                "the condition that must be met to stop keeping rows",
            )
            .filter()
    }

    fn usage(&self) -> &str {
        "Keeps rows until the condition matches."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let scope = args.call_info.scope.clone();
        let call_info = args.evaluate_once(&registry)?;

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

        let objects = call_info.input.take_while(move |item| {
            let condition = condition.clone();
            trace!("ITEM = {:?}", item);
            let result =
                evaluate_baseline_expr(&*condition, &registry, &scope.clone().set_it(item.clone()));
            trace!("RESULT = {:?}", result);

            let return_value = match result {
                Ok(ref v) if v.is_true() => false,
                _ => true,
            };

            futures::future::ready(return_value)
        });

        Ok(objects.from_input_stream())
    }
}
