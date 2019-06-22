use crate::errors::ShellError;
use crate::parser::registry::PositionalType;
use crate::parser::CommandConfig;
use crate::prelude::*;

pub struct SkipWhile;

impl Command for SkipWhile {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        skip_while(args)
    }
    fn name(&self) -> &str {
        "skip-while"
    }

    fn config(&self) -> CommandConfig {
        CommandConfig {
            name: self.name().to_string(),
            mandatory_positional: vec![PositionalType::Block("condition".to_string())],
            optional_positional: vec![],
            rest_positional: false,
            named: indexmap::IndexMap::new(),
        }
    }
}

pub fn skip_while(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Where requires a condition",
            "needs condition",
            args.name_span,
        ));
    }

    let block = args.positional[0].as_block()?;
    let input = args.input;

    let objects = input.skip_while(move |item| {
        let result = block.invoke(&item);

        let return_value = match result {
            Ok(v) if v.is_true() => true,
            _ => false,
        };

        futures::future::ready(return_value)
    });

    Ok(objects.map(|x| ReturnValue::Value(x)).boxed())
}