use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use derive_new::new;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, ReturnSuccess, Scope, Signature, SyntaxShape};

#[derive(new, Clone)]
pub struct AliasCommand {
    name: String,
    args: Vec<String>,
    block: Block,
}

impl WholeStreamCommand for AliasCommand {
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
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let call_info = args.call_info.clone();
        let registry = registry.clone();
        let block = self.block.clone();
        let alias_command = self.clone();
        let mut context = Context::from_args(&args, &registry);
        let mut input = args.input;

        let stream = async_stream! {
            while let Some(input) = input.next().await {
                let call_info = call_info.clone();
                let tag = tag.clone();
                let evaluated = call_info.evaluate_with_new_it(&registry, &input)?;
                let mut scope = Scope::it_value(input.clone());
                if let Some(positional) = &evaluated.args.positional {
                    for (pos, arg) in positional.iter().enumerate() {
                        scope = scope.set_var(alias_command.args[pos].to_string(), arg.clone());
                    }
                }

                let input_clone = Ok(input.clone());
                let input_stream = futures::stream::once(async { input_clone }).boxed().to_input_stream();

                let result = run_block(
                    &block,
                    &mut context,
                    input_stream,
                    &scope
                ).await;

                match result {
                    Ok(stream) if stream.is_empty() => {
                        yield Err(ShellError::labeled_error(
                            "Expected a block",
                            "alias needs a block",
                            tag,
                        ));
                    }
                    Ok(mut stream) => {
                        // We collect first to ensure errors are put into the context
                        while let Some(result) = stream.next().await {
                            yield Ok(ReturnSuccess::Value(result));
                        }

                        let errors = context.get_errors();
                        if let Some(x) = errors.first() {
                            //yield Err(error.clone());
                            yield Err(ShellError::labeled_error_with_secondary(
                                "Alias failed to run",
                                "alias failed to run",
                                tag.clone(),
                                x.to_string(),
                                tag
                            ));
                        }
                    }
                    Err(e) => {
                        //yield Err(e);
                        yield Err(ShellError::labeled_error_with_secondary(
                            "Alias failed to run",
                            "alias failed to run",
                            tag.clone(),
                            e.to_string(),
                            tag
                        ));
                    }
                }
            }
        };

        Ok(stream.to_output_stream())
    }
}
