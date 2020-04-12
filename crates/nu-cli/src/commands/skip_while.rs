use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, Evaluate, ReturnSuccess, Scope, Signature, SyntaxShape, Value};

pub struct SkipWhile;

impl WholeStreamCommand for SkipWhile {
    fn name(&self) -> &str {
        "skip-while"
    }

    fn signature(&self) -> Signature {
        Signature::build("skip-while")
            .required(
                "condition",
                SyntaxShape::Condition,
                "the condition that must be met to continue skipping",
            )
            .filter()
    }

    fn usage(&self) -> &str {
        "Skips rows while the condition matches."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let call_info = args.call_info.clone();
        let registry = registry.clone();
        let objects = args.input.values.skip_while(move |item| {
            let call_info = call_info.clone();
            // trace!("ITEM = {:?}", item);
            // let result = condition.invoke(&Scope::new(item.clone()));
            // trace!("RESULT = {:?}", result);

            let call_info = call_info.evaluate(&registry, &Scope::new(item.clone()));

            // FIXME, for now just swallow errors when we have an issue
            let return_value = match call_info {
                Ok(call_info) => match call_info.args.expect_nth(0) {
                    Ok(ref v) if v.is_true() => true,
                    _ => false,
                },
                _ => false,
            };

            futures::future::ready(return_value)
        });

        Ok(objects.from_input_stream())
        // let condition = args.call_info.args.expect_nth(0)?;

        // let stream = if self.is_skipping {
        //     match condition.as_bool() {
        //         Ok(b) => {
        //             if b {
        //                 VecDeque::from(vec![Ok(ReturnSuccess::Value(input))])
        //             } else {
        //                 self.is_skipping = false;
        //                 VecDeque::new()
        //             }
        //         }
        //         Err(e) => return Err(e),
        //     }
        // } else {
        //     VecDeque::from(vec![Ok(ReturnSuccess::Value(input))])
        // };

        // Ok(stream.into())
    }
}
