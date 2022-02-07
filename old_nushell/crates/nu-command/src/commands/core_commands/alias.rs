use crate::prelude::*;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};

pub struct Alias;

impl WholeStreamCommand for Alias {
    fn name(&self) -> &str {
        "alias"
    }

    fn signature(&self) -> Signature {
        Signature::build("alias")
            .required("name", SyntaxShape::String, "the name of the alias")
            .required("equals", SyntaxShape::String, "the equals sign")
            .rest("rest", SyntaxShape::Any, "the expansion for the alias")
    }

    fn usage(&self) -> &str {
        "Alias a command to an expansion."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        alias(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Alias ll to ls -l",
            example: "alias ll = ls -l",
            result: None,
        }]
    }
}

pub fn alias(args: CommandArgs) -> Result<OutputStream, ShellError> {
    // TODO: is there a better way of checking whether no arguments were passed?
    if args.nth(0).is_none() {
        let aliases = UntaggedValue::string(
            &args
                .scope()
                .get_aliases()
                .iter()
                .map(|val| format!("{} = '{}'", val.0, val.1.iter().map(|x| &x.item).join(" ")))
                .join("\n"),
        );
        return Ok(OutputStream::one(aliases));
    }
    Ok(OutputStream::empty())
}
