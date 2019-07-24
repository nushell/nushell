use crate::errors::ShellError;
use crate::object::base as value;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{self, CommandConfig, PositionalType};
use crate::prelude::*;

use futures::future::ready;
use indexmap::IndexMap;
use log::trace;

pub struct Where;

impl Command for Where {
    fn run(
        &self,
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once(registry)?;
        let condition = value::Block::extract(args.expect_nth(0)?)?;
        let input = args.input;
        let input: InputStream =
            trace_stream!(target: "nu::trace_stream::where", "where input" = input);

        Ok(input
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

    fn name(&self) -> &str {
        "where"
    }

    fn config(&self) -> CommandConfig {
        CommandConfig {
            name: self.name().to_string(),
            positional: vec![PositionalType::mandatory("condition", SyntaxType::Block)],
            rest_positional: false,
            named: IndexMap::default(),
            is_sink: true,
            is_filter: false,
        }
    }
}

// command! {
//     Where as where(args, condition: Block,) {
//         let input = args.input;
//         let input: InputStream = trace_stream!(target: "nu::trace_stream::where", "where input" = input);

//         input.values.filter_map(move |item| {
//             let result = condition.invoke(&item);

//             let return_value = match result {
//                 Err(err) => Some(Err(err)),
//                 Ok(v) if v.is_true() => Some(Ok(ReturnSuccess::Value(item.clone()))),
//                 _ => None,
//             };

//            ready(return_value)
//         })
//     }
// }
