use crate::commands::StaticCommand;
use crate::errors::ShellError;
use crate::object::base as value;
use crate::parser::hir::SyntaxType;
use crate::parser::registry;
use crate::prelude::*;

use futures::future::ready;
use serde::Deserialize;

pub struct Where;

#[derive(Deserialize)]
struct WhereArgs {
    condition: value::Block,
}

impl StaticCommand for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn signature(&self) -> registry::Signature {
        Signature::build("where")
            .required("condition", SyntaxType::Block)
            .sink()
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, run)?.run()
    }
}

fn run(
    WhereArgs { condition }: WhereArgs,
    context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(context
        .input
        .values
        .filter_map(move |item| {
            let result = condition.invoke(&item);

            let return_value = match result {
                Err(err) => Some(Err(err)),
                Ok(v) if v.is_true() => Some(Ok(ReturnSuccess::Value(item.clone()))),
                _ => None,
            };

            ready(return_value)
        })
        .boxed()
        .to_output_stream())
}
