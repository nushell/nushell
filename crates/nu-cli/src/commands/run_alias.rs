use crate::commands::classified::pipeline::run_pipeline;
use crate::prelude::*;

use derive_new::new;
use nu_errors::ShellError;
use nu_protocol::{
    hir::ClassifiedPipeline, hir::Commands, CallInfo, ReturnSuccess, Scope, Signature, SyntaxShape,
    Value,
};

#[derive(new)]
pub struct AliasCommand {
    name: String,
    args: Vec<String>,
    block: Commands,
}

impl PerItemCommand for AliasCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        let mut alias = Signature::build(&self.name);

        for arg in &self.args {
            alias = alias.optional(arg, SyntaxShape::Any, "");
        }

        alias
    }

    fn usage(&self) -> &str {
        ""
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        input: Value,
    ) -> Result<OutputStream, ShellError> {
        let tag = call_info.name_tag.clone();
        let call_info = call_info.clone();
        let registry = registry.clone();
        let raw_args = raw_args.clone();
        let block = self.block.clone();

        let mut scope = Scope::it_value(input.clone());
        if let Some(positional) = &call_info.args.positional {
            for (pos, arg) in positional.iter().enumerate() {
                scope = scope.set_var(self.args[pos].to_string(), arg.clone());
            }
        }

        let stream = async_stream! {
            let mut context = Context::from_raw(&raw_args, &registry);

            let input_clone = Ok(input.clone());
            let input_stream = futures::stream::once(async { input_clone }).boxed().to_input_stream();

            let result = run_pipeline(
                ClassifiedPipeline::new(block.clone(), None),
                &mut context,
                input_stream,
                &scope
            ).await;

            match result {
                Ok(stream) if stream.is_empty() => {
                    yield Err(ShellError::labeled_error(
                        "Expected a block",
                        "each needs a block",
                        tag,
                    ));
                }
                Ok(mut stream) => {
                    // We collect first to ensure errors are put into the context
                    while let Some(result) = stream.next().await {
                        yield Ok(ReturnSuccess::Value(result));
                    }

                    let errors = context.get_errors();
                    if let Some(error) = errors.first() {
                        yield Err(error.clone());
                    }
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        };

        Ok(stream.to_output_stream())
    }
}
