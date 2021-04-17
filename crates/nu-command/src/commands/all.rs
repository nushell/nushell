use crate::prelude::*;
use nu_engine::evaluate_baseline_expr;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{hir::CapturedBlock, hir::ClassifiedCommand, Signature, SyntaxShape};

pub struct Command;

struct AllArgs {
    predicate: CapturedBlock,
}

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "all?"
    }

    fn signature(&self) -> Signature {
        Signature::build("all?").required(
            "condition",
            SyntaxShape::RowCondition,
            "the condition that must match",
        )
    }

    fn usage(&self) -> &str {
        "Find if the table rows matches the condition."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        all(args)
    }

    fn examples(&self) -> Vec<Example> {
        use nu_protocol::Value;

        vec![
            Example {
                description: "Find if services are running",
                example: "echo [[status]; [UP] [UP]] | all? status == UP",
                result: Some(vec![Value::from(true)]),
            },
            Example {
                description: "Check that all values are even",
                example: "echo [2 4 6 8] | all? $(= $it mod 2) == 0",
                result: Some(vec![Value::from(true)]),
            },
        ]
    }
}

fn all(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = EvaluationContext::from_args(&args);
    let mut args = args.evaluate_once()?;
    let all_args = AllArgs {
        predicate: args.req(0)?,
    };

    let err = Err(ShellError::labeled_error(
        "Expected a condition",
        "expected a condition",
        args.call_info.name_tag.clone(),
    ));

    //This seems a little odd. Can't we have predicates with pipelines/multiple statements?
    let condition = {
        if all_args.predicate.block.block.len() != 1 {
            return err;
        }
        match all_args.predicate.block.block[0].pipelines.get(0) {
            Some(item) => match item.list.get(0) {
                Some(ClassifiedCommand::Expr(expr)) => expr.clone(),
                _ => {
                    return err;
                }
            },
            None => {
                return err;
            }
        }
    };

    let scope = args.scope.clone();

    // Variables in nu are immutable. Having the same variable accross invocations
    // of evaluate_baseline_expr does not mutate the variables and those each
    // invocations are independent of each other!
    scope.enter_scope();
    scope.add_vars(&all_args.predicate.captured.entries);
    let result = args.input.all(|row| {
        //$it gets overwritten each invocation
        scope.add_var("$it", row);

        let condition = evaluate_baseline_expr(&*condition, &ctx);
        match condition {
            Ok(cond) => cond.as_bool().unwrap_or(false),
            Err(_) => false,
        }
    });
    scope.exit_scope();

    Ok(OutputStream::one(result))
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
