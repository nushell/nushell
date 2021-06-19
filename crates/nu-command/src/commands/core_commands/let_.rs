use crate::prelude::*;
use nu_engine::{evaluate_baseline_expr, FromValue, WholeStreamCommand};

use nu_errors::ShellError;
use nu_protocol::{
    hir::{CapturedBlock, ClassifiedCommand},
    Signature, SyntaxShape, UntaggedValue,
};

pub struct Let;

impl WholeStreamCommand for Let {
    fn name(&self) -> &str {
        "let"
    }

    fn signature(&self) -> Signature {
        Signature::build("let")
            .required("name", SyntaxShape::String, "the name of the variable")
            .required("equals", SyntaxShape::String, "the equals sign")
            .required(
                "expr",
                SyntaxShape::MathExpression,
                "the value for the variable",
            )
    }

    fn usage(&self) -> &str {
        "Create a variable and give it a value."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        letcmd(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Assign a simple value to a variable",
                example: "let x = 3",
                result: Some(vec![]),
            },
            Example {
                description: "Assign the result of an expression to a variable",
                example: "let result = (3 + 7); echo $result",
                result: Some(vec![UntaggedValue::int(1).into()]),
            },
            Example {
                description: "Create a variable using the full name",
                example: "let $three = 3",
                result: Some(vec![]),
            },
        ]
    }
}

pub fn letcmd(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let ctx = &args.context;
    let positional = args
        .call_info
        .args
        .positional
        .expect("Internal error: type checker should require args");

    let var_name = positional[0].var_name()?;
    let rhs_raw = evaluate_baseline_expr(&positional[2], &ctx)?;
    let tag: Tag = positional[2].span.into();

    let rhs: CapturedBlock = FromValue::from_value(&rhs_raw)?;

    let (expr, _) = {
        if rhs.block.block.len() != 1 {
            return Err(ShellError::labeled_error(
                "Expected a value",
                "expected a value",
                tag,
            ));
        }
        match rhs.block.block[0].pipelines.get(0) {
            Some(item) => match item.list.get(0) {
                Some(ClassifiedCommand::Expr(expr)) => (expr, &rhs.captured),
                _ => {
                    return Err(ShellError::labeled_error(
                        "Expected a value",
                        "expected a value",
                        tag,
                    ));
                }
            },
            None => {
                return Err(ShellError::labeled_error(
                    "Expected a value",
                    "expected a value",
                    tag,
                ));
            }
        }
    };

    ctx.scope.enter_scope();
    let value = evaluate_baseline_expr(expr, &ctx);
    ctx.scope.exit_scope();

    let value = value?;

    // Note: this is a special case for setting the context from a command
    // In this case, if we don't set it now, we'll lose the scope that this
    // variable should be set into.
    ctx.scope.add_var(var_name, value);

    Ok(ActionStream::empty())
}
