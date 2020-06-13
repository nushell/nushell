use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::data::config;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    hir::Block, CommandAction, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct Alias;

#[derive(Deserialize)]
pub struct AliasArgs {
    pub name: Tagged<String>,
    pub args: Vec<Value>,
    pub block: Block,
    pub save: Option<bool>,
}

#[async_trait]
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
            .switch("save", "save the alias to your config", Some('s'))
    }

    fn usage(&self) -> &str {
        "Define a shortcut for another command."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        alias(args, registry).await
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

pub async fn alias(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let mut raw_input = args.raw_input.clone();
    let (
        AliasArgs {
            name,
            args: list,
            block,
            save,
        },
        _ctx,
    ) = args.process(&registry).await?;
    let mut processed_args: Vec<String> = vec![];

    if let Some(true) = save {
        let mut result = crate::data::config::read(name.clone().tag, &None)?;

        // process the alias to remove the --save flag
        let left_brace = raw_input.find('{').unwrap_or(0);
        let right_brace = raw_input.rfind('}').unwrap_or_else(|| raw_input.len());
        let left = raw_input[..left_brace]
            .replace("--save", "")
            .replace("-s", "");
        let right = raw_input[right_brace..]
            .replace("--save", "")
            .replace("-s", "");
        raw_input = format!("{}{}{}", left, &raw_input[left_brace..right_brace], right);

        // create a value from raw_input alias
        let alias: Value = raw_input.trim().to_string().into();
        let alias_start = raw_input.find('[').unwrap_or(0); // used to check if the same alias already exists

        // add to startup if alias doesn't exist and replce if it does
        match result.get_mut("startup") {
            Some(startup) => {
                if let UntaggedValue::Table(ref mut commands) = startup.value {
                    if let Some(command) = commands.iter_mut().find(|command| {
                        let cmd_str = command.as_string().unwrap_or_default();
                        cmd_str.starts_with(&raw_input[..alias_start])
                    }) {
                        *command = alias;
                    } else {
                        commands.push(alias);
                    }
                }
            }
            None => {
                let table = UntaggedValue::table(&[alias]);
                result.insert("startup".to_string(), table.into_value(Tag::default()));
            }
        }
        config::write(&result, &None)?;
    }

    for item in list.iter() {
        if let Ok(string) = item.as_string() {
            processed_args.push(format!("${}", string));
        } else {
            return Err(ShellError::labeled_error(
                "Expected a string",
                "expected a string",
                item.tag(),
            ));
        }
    }

    Ok(OutputStream::one(ReturnSuccess::action(
        CommandAction::AddAlias(name.to_string(), processed_args, block),
    )))
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
