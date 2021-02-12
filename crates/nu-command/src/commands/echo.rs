use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{
    Primitive, Range, RangeInclusion, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct Echo;

#[derive(Deserialize, Debug)]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        echo(args).await
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

async fn echo(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (args, _): (EchoArgs, _) = args.process().await?;

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
            x => OutputStream::one(Ok(ReturnSuccess::Value(x))),
        },
    });

    Ok(futures::stream::iter(stream).flatten().to_output_stream())
}

struct RangeIterator {
    curr: Primitive,
    end: Primitive,
    tag: Tag,
    is_end_inclusive: bool,
    moves_up: bool,
}

impl RangeIterator {
    pub fn new(range: Range, tag: Tag) -> RangeIterator {
        let start = match range.from.0.item {
            Primitive::Nothing => Primitive::Int(0.into()),
            x => x,
        };

        let end = match range.to.0.item {
            Primitive::Nothing => Primitive::Int(u64::MAX.into()),
            x => x,
        };

        RangeIterator {
            moves_up: start <= end,
            curr: start,
            end,
            tag,
            is_end_inclusive: matches!(range.to.1, RangeInclusion::Inclusive),
        }
    }
}

impl Iterator for RangeIterator {
    type Item = Result<ReturnSuccess, ShellError>;
    fn next(&mut self) -> Option<Self::Item> {
        let ordering = if self.end == Primitive::Nothing {
            Ordering::Less
        } else {
            let result =
                nu_data::base::coerce_compare_primitive(&self.curr, &self.end).map_err(|_| {
                    ShellError::labeled_error(
                        "Cannot create range",
                        "unsupported range",
                        self.tag.span,
                    )
                });

            if let Err(result) = result {
                return Some(Err(result));
            }

            let result = result
                .expect("Internal error: the error case was already protected, but that failed");

            result.compare()
        };

        use std::cmp::Ordering;

        if self.moves_up
            && (ordering == Ordering::Less || self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let output = UntaggedValue::Primitive(self.curr.clone()).into_value(self.tag.clone());

            let next_value = nu_data::value::compute_values(
                Operator::Plus,
                &UntaggedValue::Primitive(self.curr.clone()),
                &UntaggedValue::int(1),
            );

            self.curr = match next_value {
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
        } else if !self.moves_up
            && (ordering == Ordering::Greater
                || self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let output = UntaggedValue::Primitive(self.curr.clone()).into_value(self.tag.clone());

            let next_value = nu_data::value::compute_values(
                Operator::Plus,
                &UntaggedValue::Primitive(self.curr.clone()),
                &UntaggedValue::int(-1),
            );

            self.curr = match next_value {
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
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Echo;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Echo {})
    }
}
