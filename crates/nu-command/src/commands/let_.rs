use crate::prelude::*;
use nu_engine::{evaluate_baseline_expr, WholeStreamCommand};

use nu_errors::ShellError;
use nu_protocol::{hir::CapturedBlock, hir::ClassifiedCommand, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct Let;

#[derive(Deserialize)]
pub struct LetArgs {
    pub name: Tagged<String>,
    pub equals: Tagged<String>,
    pub rhs: CapturedBlock,
}

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        letcmd(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

pub async fn letcmd(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let ctx = EvaluationContext::from_args(&args);

    let (LetArgs { name, rhs, .. }, _) = args.process().await?;

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

    // Note: this is a special case for setting the context from a command
    // In this case, if we don't set it now, we'll lose the scope that this
    // variable should be set into.
    ctx.scope.add_var(name, value);

    Ok(OutputStream::empty())
}
