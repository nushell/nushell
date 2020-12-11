use crate::prelude::*;
use crate::{commands::WholeStreamCommand, evaluate::evaluate_baseline_expr};

use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct Set;

#[derive(Deserialize)]
pub struct SetArgs {
    pub name: Tagged<String>,
    pub equals: Tagged<String>,
    pub rest: Vec<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Set {
    fn name(&self) -> &str {
        "set"
    }

    fn signature(&self) -> Signature {
        Signature::build("set")
            .required("name", SyntaxShape::String, "the name of the variable")
            .required("=", SyntaxShape::String, "the equals sign")
            .rest(SyntaxShape::String, "the value to set the variable to")
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
    let mut scope = args.scope.clone();
    let ctx = EvaluationContext::from_args(&args);
    let (SetArgs { name, equals, rest }, input) = args.process().await?;

    let strings: Vec<_> = rest
        .into_iter()
        .map(|x| {
            let span = x.span();
            x.item.spanned(span)
        })
        .collect();

    let (_, expr, err) = nu_parser::parse_math_expression(0, &strings, &scope, false);
    if let Some(err) = err {
        return Err(err.into());
    }

    let value = evaluate_baseline_expr(&expr, &ctx).await?;

    let name = if name.item.starts_with('$') {
        name.item.clone()
    } else {
        format!("${}", name.item)
    };

    Ok(OutputStream::one(ReturnSuccess::action(
        CommandAction::AddVariable(name, value),
    )))
}
