use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::data::config;
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

pub fn alias(alias_args: AliasArgs, _: RunnableContext) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let mut args: Vec<String> = vec![];
        // let name_span = args.name.clone();
        // let mut result = crate::data::config::read(name_span.tag, &None)?;

        // let value = Value {
        //     value: UntaggedValue::Block(block.clone()),
        //     tag: Tag::default(),
        // };
        // result.insert(String::from("startup"), value);

        // config::write(&result, &None)?;
        // TODO fix printing of alias_args
        // println!("{:#?}", alias_args.block);
        for item in alias_args.args.iter() {
            if let Ok(string) = item.as_string() {
                args.push(format!("${}", string));
            } else {
                yield Err(ShellError::labeled_error("Expected a string", "expected a string", item.tag()));
            }
        }
        println!("alias {} {:?} {}", alias_args.name.to_string(), args, alias_args.block);
        yield ReturnSuccess::action(CommandAction::AddAlias(alias_args.name.to_string(), args, alias_args.block.clone()))
    };

    Ok(stream.to_output_stream())
}
