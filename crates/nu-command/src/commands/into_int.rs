use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

use num_bigint::ToBigInt;

pub struct IntoInt;

#[derive(Deserialize)]
pub struct IntoIntArgs {
    pub rest: Vec<Value>,
}

#[async_trait]
impl WholeStreamCommand for IntoInt {
    fn name(&self) -> &str {
        "into-int"
    }

    fn signature(&self) -> Signature {
        Signature::build("into-int").rest(SyntaxShape::Any, "the values to into-int")
    }

    fn usage(&self) -> &str {
        "Convert value to integer"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        into_int(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert filesize to integer",
                example: "into-int 1kb | each { = $it / 1000 }",
                result: Some(vec![UntaggedValue::int(1).into()]),
            },
            Example {
                description: "Convert filesize to integer",
                example: "into-int 1kib | each { = $it / 1024 }",
                result: Some(vec![UntaggedValue::int(1).into()]),
            },
        ]
    }
}

async fn into_int(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (args, _): (IntoIntArgs, _) = args.process().await?;

    let stream = args.rest.into_iter().map(|i| match i {
        Value {
            value: UntaggedValue::Primitive(primitive_val),
            tag,
        } => match primitive_val {
            Primitive::Filesize(size) => OutputStream::one(Ok(ReturnSuccess::Value(Value {
                value: UntaggedValue::int(size.to_bigint().expect("Conversion should never fail.")),
                tag,
            }))),
            Primitive::Int(_) => OutputStream::one(Ok(ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(primitive_val),
                tag,
            }))),
            _ => OutputStream::one(Err(ShellError::labeled_error(
                "Could not convert int value",
                "original value",
                tag,
            ))),
        },
        _ => OutputStream::one(Ok(ReturnSuccess::Value(i))),
    });

    Ok(futures::stream::iter(stream).flatten().to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::IntoInt;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(IntoInt {})
    }
}
