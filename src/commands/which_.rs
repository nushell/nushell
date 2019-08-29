use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;

use crate::commands::WholeStreamCommand;
use crate::parser::registry::Signature;

pub struct Which;

impl WholeStreamCommand for Which {
    fn name(&self) -> &str {
        "which"
    }

    fn signature(&self) -> Signature {
        Signature::build("which")
            .required("name", SyntaxType::Any)
    }

    fn usage(&self) -> &str {
        "Finds a program file."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        which(args, registry)
    }
}

pub fn which(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    let mut which_out = VecDeque::new();
    let span = args.call_info.name_span;

    if let Some(v) = &args.call_info.args.positional {
        if v.len() > 0 {
            match &v[0] {
                Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    tag,
                } => match which::which(&s) {
                    Ok(ok) => {
                        which_out
                            .push_back(Value::Primitive(Primitive::Path(ok)).tagged(tag.clone()));
                    }
                    _ => {}
                },
                Tagged { tag, .. } => {
                    return Err(ShellError::labeled_error(
                        "Expected a filename to find",
                        "needs a filename",
                        tag.span,
                    ));
                }
            }
        } else {
            return Err(ShellError::labeled_error(
                "Expected a binary to find",
                "needs application name",
                span,
            ));
        }
    } else {
        return Err(ShellError::labeled_error(
            "Expected a binary to find",
            "needs application name",
            span,
        ));
    }

    Ok(which_out.to_output_stream())
}
