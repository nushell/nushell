use crate::prelude::*;
use crate::{commands::WholeStreamCommand, evaluate::evaluate_baseline_expr};

use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ClassifiedCommand, CommandAction, ReturnSuccess, Signature,
    SyntaxShape,
};
use nu_source::Tagged;

pub struct Set;

#[derive(Deserialize)]
pub struct SetArgs {
    pub name: Tagged<String>,
    pub equals: Tagged<String>,
    pub rhs: CapturedBlock,
}

#[async_trait]
impl WholeStreamCommand for Set {
    fn name(&self) -> &str {
        "set"
    }

    fn signature(&self) -> Signature {
        Signature::build("set")
            .required("name", SyntaxShape::String, "the name of the variable")
            .required("equals", SyntaxShape::String, "the equals sign")
            .required(
                "expr",
                SyntaxShape::MathExpression,
                "the value to set the variable to",
            )
    }

    fn usage(&self) -> &str {
        "Create a variable and set it to a value."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        set(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

pub async fn set(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let ctx = EvaluationContext::from_args(&args);

    let (SetArgs { name, rhs, .. }, _) = args.process().await?;

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

    let name = if name.item.starts_with('$') {
        name.item.clone()
    } else {
        format!("${}", name.item)
    };

    Ok(OutputStream::one(ReturnSuccess::action(
        CommandAction::AddVariable(name, value),
    )))
}
