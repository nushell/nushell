use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{
    Primitive, Range, RangeInclusion, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct Echo;

#[derive(Deserialize)]
pub struct EchoArgs {
    pub rest: Vec<Value>,
}

#[async_trait]
impl WholeStreamCommand for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn signature(&self) -> Signature {
        Signature::build("echo").rest(SyntaxShape::Any, "the values to echo")
    }

    fn usage(&self) -> &str {
        "Echo the arguments back to the user."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        echo(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Put a hello message in the pipeline",
                example: "echo 'hello'",
                result: Some(vec![Value::from("hello")]),
            },
            Example {
                description: "Print the value of the special '$nu' variable",
                example: "echo $nu",
                result: None,
            },
        ]
    }
}

async fn echo(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (args, _): (EchoArgs, _) = args.process(&registry).await?;

    let stream = args.rest.into_iter().map(|i| match i.as_string() {
        Ok(s) => OutputStream::one(Ok(ReturnSuccess::Value(
            UntaggedValue::string(s).into_value(i.tag.clone()),
        ))),
        _ => match i {
            Value {
                value: UntaggedValue::Table(table),
                ..
            } => futures::stream::iter(table.into_iter().map(ReturnSuccess::value))
                .to_output_stream(),
            Value {
                value: UntaggedValue::Primitive(Primitive::Range(range)),
                tag,
            } => futures::stream::iter(RangeIterator::new(*range, tag)).to_output_stream(),
            _ => OutputStream::one(Ok(ReturnSuccess::Value(i.clone()))),
        },
    });

    Ok(futures::stream::iter(stream).flatten().to_output_stream())
}

struct RangeIterator {
    curr: Primitive,
    end: Primitive,
    tag: Tag,
    is_end_inclusive: bool,
    is_done: bool,
}

impl RangeIterator {
    pub fn new(range: Range, tag: Tag) -> RangeIterator {
        RangeIterator {
            curr: range.from.0.item,
            end: range.to.0.item,
            tag,
            is_end_inclusive: matches!(range.to.1, RangeInclusion::Inclusive),
            is_done: false,
        }
    }
}

impl Iterator for RangeIterator {
    type Item = Result<ReturnSuccess, ShellError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr != self.end {
            let output = UntaggedValue::Primitive(self.curr.clone()).into_value(self.tag.clone());

            self.curr = match crate::data::value::compute_values(
                Operator::Plus,
                &UntaggedValue::Primitive(self.curr.clone()),
                &UntaggedValue::int(1),
            ) {
                Ok(result) => match result {
                    UntaggedValue::Primitive(p) => p,
                    _ => {
                        return Some(Err(ShellError::unimplemented(
                            "Internal error: expected a primitive result from increment",
                        )));
                    }
                },
                Err((left_type, right_type)) => {
                    return Some(Err(ShellError::coerce_error(
                        left_type.spanned(self.tag.span),
                        right_type.spanned(self.tag.span),
                    )));
                }
            };
            Some(ReturnSuccess::value(output))
        } else if self.is_end_inclusive && !self.is_done {
            self.is_done = true;
            Some(ReturnSuccess::value(
                UntaggedValue::Primitive(self.curr.clone()).into_value(self.tag.clone()),
            ))
        } else {
            // TODO: add inclusive/exclusive ranges
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Echo;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Echo {})
    }
}
