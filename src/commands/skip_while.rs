use crate::errors::ShellError;
use crate::parser::registry::CommandConfig;
use crate::parser::registry::PositionalType;
use crate::prelude::*;

pub struct SkipWhile;

impl Command for SkipWhile {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        skip_while(args, registry)
    }
    fn name(&self) -> &str {
        "skip-while"
    }

    fn config(&self) -> CommandConfig {
        CommandConfig {
            name: self.name().to_string(),
            positional: vec![PositionalType::mandatory_block("condition")],
            rest_positional: false,
            named: indexmap::IndexMap::new(),
            is_filter: true,
            is_sink: false,
        }
    }
}

pub fn skip_while(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let block = args.expect_nth(0)?.as_block()?;
    let span = args.name_span();
    let len = args.len();
    let input = args.input;

    if len == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Where requires a condition",
            "needs condition",
            span,
        ));
    }

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
