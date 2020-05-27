use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{
    Primitive, RangeInclusion, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct Echo;

#[derive(Deserialize)]
pub struct EchoArgs {
    pub rest: Vec<Value>,
}

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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        echo(args, registry)
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

fn echo(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (args, _): (EchoArgs, _) = args.process(&registry).await?;

        for i in args.rest {
            match i.as_string() {
                Ok(s) => {
                    yield Ok(ReturnSuccess::Value(
                        UntaggedValue::string(s).into_value(i.tag.clone()),
                    ));
                }
                _ => match i {
                    Value {
                        value: UntaggedValue::Table(table),
                        ..
                    } => {
                        for value in table {
                            yield Ok(ReturnSuccess::Value(value.clone()));
                        }
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Range(range)),
                        tag
                    } => {
                        let mut current = range.from.0.item;
                        while current != range.to.0.item {
                            yield Ok(ReturnSuccess::Value(UntaggedValue::Primitive(current.clone()).into_value(&tag)));
                            current = match crate::data::value::compute_values(Operator::Plus, &UntaggedValue::Primitive(current), &UntaggedValue::int(1)) {
                                Ok(result) => match result {
                                    UntaggedValue::Primitive(p) => p,
                                    _ => {
                                        yield Err(ShellError::unimplemented("Internal error: expected a primitive result from increment"));
                                        return;
                                    }
                                },
                                Err((left_type, right_type)) => {
                                    yield Err(ShellError::coerce_error(
                                        left_type.spanned(tag.span),
                                        right_type.spanned(tag.span),
                                    ));
                                    return;
                                }
                            }
                        }
                        match range.to.1 {
                            RangeInclusion::Inclusive => {
                                yield Ok(ReturnSuccess::Value(UntaggedValue::Primitive(current.clone()).into_value(&tag)));
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        yield Ok(ReturnSuccess::Value(i.clone()));
                    }
                },
            }
        }
    };

    Ok(stream.to_output_stream())
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
