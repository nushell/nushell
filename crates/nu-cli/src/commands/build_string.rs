use crate::prelude::*;
use nu_errors::ShellError;

use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

#[derive(Deserialize)]
pub struct BuildStringArgs {
    rest: Vec<Value>,
}

pub struct BuildString;

#[async_trait]
impl WholeStreamCommand for BuildString {
    fn name(&self) -> &str {
        "build-string"
    }

    fn signature(&self) -> Signature {
        Signature::build("build-string")
            .rest(SyntaxShape::Any, "all values to form into the string")
    }

    fn usage(&self) -> &str {
        "Builds a string from the arguments"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (BuildStringArgs { rest }, _) = args.process(&registry).await?;

        let mut output_string = String::new();

        for r in rest {
            output_string.push_str(&format_leaf(&r).plain_string(100_000))
        }

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(output_string).into_value(tag),
        )))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Builds a string from a string and a number, without spaces between them",
            example: "build-string 'foo' 3",
            result: None,
        }]
    }
}
