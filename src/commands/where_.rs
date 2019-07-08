use crate::errors::ShellError;
use crate::object::Block;
use crate::prelude::*;
use futures::future::ready;
use log::trace;

command! {
    Where as where(args, condition: Block) {
        let input: InputStream = trace_stream!("where input" = args.input);

        input.values.filter_map(move |item| {
            let result = condition.invoke(&item);

            let return_value = match result {
                Err(err) => Some(Err(err)),
                Ok(v) if v.is_true() => Some(Ok(ReturnSuccess::Value(item.clone()))),
                _ => None,
            };

           ready(return_value)
        })
    }
}
