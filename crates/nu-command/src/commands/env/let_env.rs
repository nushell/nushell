use std::convert::TryInto;

use crate::prelude::*;
use nu_engine::{evaluate_baseline_expr, EnvVar, WholeStreamCommand};

use nu_errors::ShellError;
use nu_protocol::{hir::CapturedBlock, hir::ClassifiedCommand, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct LetEnv;

#[derive(Deserialize)]
pub struct LetEnvArgs {
    pub name: Tagged<String>,
    pub equals: Tagged<String>,
    pub rhs: CapturedBlock,
}

impl WholeStreamCommand for LetEnv {
    fn name(&self) -> &str {
        "let-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("let-env")
            .required(
                "name",
                SyntaxShape::String,
                "the name of the environment variable",
            )
            .required("equals", SyntaxShape::String, "the equals sign")
            .required(
                "expr",
                SyntaxShape::MathExpression,
                "the value for the environment variable",
            )
    }

    fn usage(&self) -> &str {
        "Create an environment variable and give it a value."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        set_env(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

pub fn set_env(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let ctx = &args.context;

    let name: Tagged<String> = args.req(0)?;
    let rhs: CapturedBlock = args.req(2)?;

    let (expr, captured) = {
        if rhs.block.block.len() != 1 {
            return Err(ShellError::labeled_error(
                "Expected a value",
                "expected a value",
                tag,
            ));
        }
        match rhs.block.block[0].pipelines.get(0) {
            Some(item) => match item.list.get(0) {
                Some(ClassifiedCommand::Expr(expr)) => (expr.clone(), rhs.captured.clone()),
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
    ctx.scope.add_vars(&captured.entries);

    let value = evaluate_baseline_expr(&expr, ctx);

    ctx.scope.exit_scope();

    let value: EnvVar = value?.try_into()?;
    let name = name.item;

    // Note: this is a special case for setting the context from a command
    // In this case, if we don't set it now, we'll lose the scope that this
    // variable should be set into.
    ctx.scope.add_env_var(name, value);

    Ok(ActionStream::empty())
}
