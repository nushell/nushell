use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::{evaluate, fetch};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::{SpannedItem, Tagged};
use nu_value_ext::ValueExt;

pub struct EvaluateBy;

#[derive(Deserialize)]
pub struct EvaluateByArgs {
    evaluate_with: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for EvaluateBy {
    fn name(&self) -> &str {
        "evaluate-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("evaluate-by").named(
            "evaluate_with",
            SyntaxShape::String,
            "the name of the column to evaluate by",
            Some('w'),
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the tables rows evaluated by the column given."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        evaluate_by(args, registry).await
    }
}

pub async fn evaluate_by(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (EvaluateByArgs { evaluate_with }, mut input) = args.process(&registry).await?;
    let values: Vec<Value> = input.collect().await;

    if values.is_empty() {
        Err(ShellError::labeled_error(
            "Expected table from pipeline",
            "requires a table input",
            name,
        ))
    } else {
        let evaluate_with = if let Some(evaluator) = evaluate_with {
            Some(evaluator.item().clone())
        } else {
            None
        };

        match evaluate(&values[0], evaluate_with, name) {
            Ok(evaluated) => Ok(OutputStream::one(ReturnSuccess::value(evaluated))),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EvaluateBy;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(EvaluateBy {})
    }
}
