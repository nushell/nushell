use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, CommandAction, ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct Alias;

#[derive(Deserialize)]
pub struct AliasArgs {
    pub name: Tagged<String>,
    pub args: Vec<Value>,
    pub block: Block,
}

impl WholeStreamCommand for Alias {
    fn name(&self) -> &str {
        "alias"
    }

    fn signature(&self) -> Signature {
        Signature::build("alias")
            .required("name", SyntaxShape::String, "the name of the alias")
            .required("args", SyntaxShape::Table, "the arguments to the alias")
            .required(
                "block",
                SyntaxShape::Block,
                "the block to run as the body of the alias",
            )
    }

    fn usage(&self) -> &str {
        "Define a shortcut for another command."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        //args.process(registry, alias)?.run()
        alias(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "An alias without parameters",
                example: "alias say-hi [] { echo 'Hello!' }",
                result: None,
            },
            Example {
                description: "An alias with a single parameter",
                example: "alias l [x] { ls $x }",
                result: None,
            },
        ]
    }
}

pub fn alias(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (AliasArgs { name, args: list, block }, _) = args.process(&registry).await?;
        let mut args: Vec<String> = vec![];
        for item in list.iter() {
            if let Ok(string) = item.as_string() {
                args.push(format!("${}", string));
            } else {
                yield Err(ShellError::labeled_error("Expected a string", "expected a string", item.tag()));
            }
        }
        yield ReturnSuccess::action(CommandAction::AddAlias(name.to_string(), args, block.clone()))
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Alias;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Alias {})
    }
}
