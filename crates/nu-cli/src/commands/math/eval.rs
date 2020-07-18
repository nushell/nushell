use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct SubCommandArgs {
    expression: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math eval"
    }

    fn usage(&self) -> &str {
        "Evaluate a math expression into a number"
    }

    fn signature(&self) -> Signature {
        Signature::build("math eval").desc(self.usage()).optional(
            "math expression",
            SyntaxShape::String,
            "the math expression to evaluate",
        )
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        eval(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Evalulate math in the pipeline",
            example: "echo '10 / 4' | math eval",
            result: Some(vec![UntaggedValue::decimal(2.5).into()]),
        }]
    }
}

pub async fn eval(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.span;
    let (SubCommandArgs { expression }, input) = args.process(registry).await?;

    Ok(input
        .map(move |x| {
            if let Some(Tagged {
                tag,
                item: expression,
            }) = &expression
            {
                UntaggedValue::string(expression).into_value(tag)
            } else {
                x
            }
        })
        .map(move |input| {
            if let Ok(string) = input.as_string() {
                match parse(&string, &input.tag) {
                    Ok(value) => ReturnSuccess::value(value),
                    Err(err) => Err(ShellError::labeled_error(
                        "Math evaluation error",
                        err,
                        &input.tag.span,
                    )),
                }
            } else {
                Err(ShellError::labeled_error(
                    "Expected a string from pipeline",
                    "requires string input",
                    name,
                ))
            }
        })
        .to_output_stream())
}

pub fn parse<T: Into<Tag>>(math_expression: &str, tag: T) -> Result<Value, String> {
    match meval::eval_str(math_expression) {
        Ok(num) if num.is_infinite() || num.is_nan() => Err("cannot represent result".to_string()),
        Ok(num) => Ok(UntaggedValue::from(Primitive::from(num)).into_value(tag)),
        Err(error) => Err(error.to_string().to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
