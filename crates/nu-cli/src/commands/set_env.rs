use crate::prelude::*;
use crate::{commands::WholeStreamCommand, evaluate::evaluate_baseline_expr};

use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ClassifiedCommand, CommandAction, ReturnSuccess, Signature,
    SyntaxShape,
};
use nu_source::Tagged;

pub struct SetEnv;

#[derive(Deserialize)]
pub struct SetEnvArgs {
    pub name: Tagged<String>,
    pub equals: Tagged<String>,
    pub rhs: CapturedBlock,
}

#[async_trait]
impl WholeStreamCommand for SetEnv {
    fn name(&self) -> &str {
        "set-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("set-env")
            .required(
                "name",
                SyntaxShape::String,
                "the name of the environment variable",
            )
            .required("equals", SyntaxShape::String, "the equals sign")
            .required(
                "expr",
                SyntaxShape::MathExpression,
                "the value to set the environment variable to",
            )
    }

    fn usage(&self) -> &str {
        "Create an environment variable and set it to a value."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        set_env(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

pub async fn set_env(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let ctx = EvaluationContext::from_args(&args);

    let (SetEnvArgs { name, rhs, .. }, _) = args.process().await?;

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

    let value = evaluate_baseline_expr(&expr, &ctx).await;

    ctx.scope.exit_scope();

    let value = value?;
    let value = value.as_string()?;

    let name = name.item.clone();

    Ok(OutputStream::one(ReturnSuccess::action(
        CommandAction::AddEnvVariable(name, value),
    )))
}
