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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, alias)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "An alias without parameters",
                example: "alias say-hi [] { echo 'Hello!' }",
            },
            Example {
                description: "An alias with a single parameter",
                example: "alias l [x] { ls $x }",
            },
        ]
    }
}

pub fn alias(alias_args: AliasArgs, ctx: RunnableContext) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let mut args: Vec<String> = vec![];

        if let Some(true) = alias_args.save {
            let mut result = crate::data::config::read(alias_args.name.clone().tag, &None)?;

            let mut raw_input = ctx.raw_input.clone();
            let left_brace = raw_input.find('{').unwrap();
            let right_brace = raw_input.rfind('}').unwrap();

            let mut left = raw_input[..left_brace].replace("--save", "");
            left = left.replace("-s", "");

            let mut right = raw_input[right_brace..].replace("--save", "");
            right = right.replace("-s", "");

            raw_input = format!("{}{}{}", left, &raw_input[left_brace..right_brace], right);
            let alias: Value = raw_input.trim().to_string().into();
            // process the alias to remove the --save flag

            // TODO remove the --save from the command
            // TODO fix partialeq impl for value
            match result.get_mut("startup") {
                Some(startup) => {
                    if let UntaggedValue::Table(ref mut commands) = startup.value {
                        if commands.iter().find(|val| {
                            val.value == alias.value
                        }).is_none() {
                            commands.push(alias);
                        }
                    }
                }
                None => {
                    let mut table = UntaggedValue::table(&[alias]);
                    result.insert("startup".to_string(), table.into_value(Tag::default()));
                }
            }
            config::write(&result, &None)?;
        }

        for item in alias_args.args.iter() {
            if let Ok(string) = item.as_string() {
                args.push(format!("${}", string));
            } else {
                yield Err(ShellError::labeled_error("Expected a string", "expected a string", item.tag()));
            }
        }
        yield ReturnSuccess::action(CommandAction::AddAlias(alias_args.name.to_string(), args, alias_args.block.clone()))
    };

    Ok(stream.to_output_stream())
}
