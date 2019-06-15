use crate::errors::ShellError;
use crate::parser::registry::PositionalType;
use crate::parser::CommandConfig;
use crate::prelude::*;

pub struct Where;

impl Command for Where {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        r#where(args)
    }
    fn name(&self) -> &str {
        "where"
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

pub fn r#where(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Where requires a condition",
            "needs condition",
            args.name_span,
        ));
    }

    let block = args.positional[0].as_block()?;
    let input = args.input;

    let objects = input.filter_map(move |item| {
        let result = block.invoke(&item);

        let return_value = match result {
            Err(err) => Some(ReturnValue::Value(Value::Error(Box::new(err)))),
            Ok(v) if v.is_true() => Some(ReturnValue::Value(item.copy())),
            _ => None,
        };

        futures::future::ready(return_value)
    });

    Ok(objects.boxed())
}
