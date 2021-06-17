use crate::prelude::*;
use nu_engine::evaluate_baseline_expr;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ClassifiedCommand, Signature, SyntaxShape, UntaggedValue,
};

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
                example: "echo [2 4 6 8] | all? ($it mod 2) == 0",
                result: Some(vec![Value::from(true)]),
            },
        ]
    }
}

fn all(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = &args.context;
    let tag = args.call_info.name_tag.clone();
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

    let scope = args.scope().clone();

    let init = Ok(InputStream::one(
        UntaggedValue::boolean(true).into_value(&tag),
    ));

    // Variables in nu are immutable. Having the same variable across invocations
    // of evaluate_baseline_expr does not mutate the variables and those each
    // invocations are independent of each other!
    scope.enter_scope();
    scope.add_vars(&all_args.predicate.captured.entries);
    let result = args.input.fold(init, move |acc, row| {
        let condition = condition.clone();
        let ctx = ctx.clone();
        ctx.scope.add_var("$it", row);

        let condition = evaluate_baseline_expr(&condition, &ctx);

        let curr = acc?.drain_vec();
        let curr = curr
            .get(0)
            .ok_or_else(|| ShellError::unexpected("No value to check with"))?;
        let cond = curr.as_bool()?;

        match condition {
            Ok(condition) => match condition.as_bool() {
                Ok(b) => Ok(InputStream::one(
                    UntaggedValue::boolean(cond && b).into_value(&curr.tag),
                )),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    });
    scope.exit_scope();

    Ok(result?.into_output_stream())
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
