use crate::prelude::*;
use nu_engine::evaluate_baseline_expr;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{hir::CapturedBlock, hir::ClassifiedCommand, Signature, SyntaxShape};
use std::error::Error;

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
    //`try_all` is like `all`, but with short-circuiting for Err
    let result: Result<bool, ShellError> = args.input.try_all(|row| {
        //$it gets overwritten each invocation
        scope.add_var("$it", row);

        let condition = evaluate_baseline_expr(&*condition, &ctx)?;

        condition.as_bool()
    });
    scope.exit_scope();

    Ok(OutputStream::one(result?))
}

trait TryAllExt: Iterator {
    /// Tests if every element of the iterator matches a predicate.
    ///
    /// `try_all()` takes a closure that returns `Ok(true)`, `Ok(false)` or Err(E). It applies
    /// this closure to each element of the iterator, and if they all return
    /// `Ok(true)`, then so does `try_all()`. If any of them return `Ok(false)`, it
    /// returns `Ok(false)`. If the closure returns Err(E), `try_all` returns Err(E) immediatly
    /// (short-circuiting).
    ///
    /// `try_all()` is short-circuiting; in other words, it will stop processing
    /// as soon as it finds a `Ok(false)` or `Err(E)`
    ///
    /// An empty iterator returns `Ok(true)`.
    fn try_all<F, E>(&mut self, mut f: F) -> Result<bool, E>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Result<bool, E>,
        E: Error,
    {
        for item in self {
            //if f fails, we return failure
            if !f(item)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl<I: Iterator> TryAllExt for I {}

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
