use crate::errors::ShellError;
use crate::parser::registry::CommandConfig;
use crate::parser::registry::PositionalType;
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
            positional: vec![PositionalType::mandatory("condition", "Block")],
            rest_positional: false,
            named: indexmap::IndexMap::new(),
            is_filter: true,
            is_sink: false,
            can_load: vec![],
            can_save: vec![],
        }
    }
}

pub fn skip_while(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Where requires a condition",
            "needs condition",
            args.name_span,
        ));
    }

    let block = args.nth(0).unwrap().as_block()?;
    let input = args.input;

    let objects = input.values.skip_while(move |item| {
        let result = block.invoke(&item);

        let return_value = match result {
            Ok(v) if v.is_true() => true,
            _ => false,
        };

        futures::future::ready(return_value)
    });

    Ok(objects.from_input_stream())
}
