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

impl WholeStreamCommand for EvaluateBy {
    fn name(&self) -> &str {
        "evaluate-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("evaluate-by").named(
            "evaluate_with",
            SyntaxShape::String,
            "the name of the column to evaluate by",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the tables rows evaluated by the column given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, evaluate_by)?.run()
    }
}

pub fn evaluate_by(
    EvaluateByArgs { evaluate_with }: EvaluateByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        if values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    name
                ))
        } else {

            let evaluate_with = if let Some(evaluator) = evaluate_with {
                Some(evaluator.item().clone())
            } else {
                None
            };

            match evaluate(&values[0], evaluate_with, name) {
                Ok(evaluated) => yield ReturnSuccess::value(evaluated),
                Err(err) => yield Err(err)
            }
        }
    };

    Ok(stream.to_output_stream())
}
