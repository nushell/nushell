use crate::prelude::*;
use nu_engine::WholeStreamCommand;

use crate::script::print_err;
use nu_errors::ShellError;
use nu_parser::expand_path;
use nu_protocol::{Primitive, Signature, SpannedTypeName, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Source;

#[derive(Deserialize)]
pub struct SourceArgs {
    pub filename: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Source {
    fn name(&self) -> &str {
        "source"
    }

    fn signature(&self) -> Signature {
        Signature::build("source").optional(
            "filename",
            SyntaxShape::String,
            "the filepath to the script file to source",
        )
    }

    fn usage(&self) -> &str {
        "Runs scripts in the current context."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        source(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Source piped input",
                example: "echo '= 41 + 1' | source ",
                result: None,
            },
            Example {
                description: "Source file",
                example: "source test.nu",
                result: None,
            },
        ]
    }
}

pub async fn source(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = EvaluationContext::from_args(&args);
    let (SourceArgs { filename }, input) = args.process().await?;

    // Take each string of the piped-input and run it as a script
    for command in input.into_vec().await {
        match command.value {
            UntaggedValue::Primitive(Primitive::String(contents)) => {
                let result =
                    crate::script::run_script_standalone(contents, true, &ctx, false).await;

                if let Err(err) = result {
                    ctx.error(err.into());
                }
            }
            _ => {
                // We print the type-errrors ourselfs because writing it to ctx.error
                // will print it at a later point possibly with a wrong source line
                // if something gets run afterwards e.g. > echo 3 ls | source
                // or only print one error even if multiple occured
                print_err(
                    ShellError::type_error("String", command.spanned_type_name()),
                    &Text::from(command.convert_to_string()),
                    &ctx,
                );
            }
        }
    }

    let filename = if let Some(name) = filename {
        name
    } else {
        return Ok(OutputStream::empty());
    };

    // Note: this is a special case for setting the context from a command
    // In this case, if we don't set it now, we'll lose the scope that this
    // variable should be set into.
    let contents = std::fs::read_to_string(expand_path(&filename.item).into_owned());
    match contents {
        Ok(contents) => {
            let result = crate::script::run_script_standalone(contents, true, &ctx, false).await;

            if let Err(err) = result {
                ctx.error(err.into());
            }
            Ok(OutputStream::empty())
        }
        Err(_) => {
            ctx.error(ShellError::labeled_error(
                "Can't load file to source",
                "can't load file",
                filename.span(),
            ));

            Ok(OutputStream::empty())
        }
    }
}
