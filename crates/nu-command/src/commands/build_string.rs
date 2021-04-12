use crate::prelude::*;
use nu_errors::ShellError;

use nu_data::value::format_leaf;
use nu_engine::WholeStreamCommand;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

pub struct BuildString;

impl WholeStreamCommand for BuildString {
    fn name(&self) -> &str {
        "build-string"
    }

    fn signature(&self) -> Signature {
        Signature::build("build-string")
            .rest(SyntaxShape::Any, "all values to form into the string")
    }

    fn usage(&self) -> &str {
        "Builds a string from the arguments."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let args = args.evaluate_once()?;
        let rest: Vec<Value> = args.rest(0)?;

        let mut output_string = String::new();

        for r in rest {
            output_string.push_str(&format_leaf(&r).plain_string(100_000))
        }

        Ok(ActionStream::one(ReturnSuccess::value(
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
