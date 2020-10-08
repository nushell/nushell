use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use derive_new::new;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, Scope, Signature, SyntaxShape, UntaggedValue};

#[derive(new, Clone)]
pub struct AliasCommand {
    name: String,
    args: Vec<(String, SyntaxShape)>,
    block: Block,
}

#[async_trait]
impl WholeStreamCommand for AliasCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        let mut alias = Signature::build(&self.name);

        for (arg, shape) in &self.args {
            alias = alias.optional(arg, *shape, "");
        }

        alias
    }

    fn usage(&self) -> &str {
        ""
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let call_info = args.call_info.clone();
        let registry = registry.clone();
        let mut block = self.block.clone();
        block.set_redirect(call_info.args.external_redirection);

        let alias_command = self.clone();
        let mut context = EvaluationContext::from_args(&args, &registry);
        let input = args.input;

        let scope = call_info.scope.clone();
        let evaluated = call_info.evaluate(&registry).await?;

        let mut vars = IndexMap::new();

        let mut num_positionals = 0;
        if let Some(positional) = &evaluated.args.positional {
            num_positionals = positional.len();
            for (pos, arg) in positional.iter().enumerate() {
                vars.insert(alias_command.args[pos].0.to_string(), arg.clone());
            }
        }

        if alias_command.args.len() > num_positionals {
            for idx in 0..(alias_command.args.len() - num_positionals) {
                vars.insert(
                    alias_command.args[idx + num_positionals].0.to_string(),
                    UntaggedValue::nothing().into_untagged_value(),
                );
            }
        }

        let scope = Scope::append_vars(scope, vars);

        // FIXME: we need to patch up the spans to point at the top-level error
        Ok(run_block(&block, &mut context, input, scope)
            .await?
            .to_output_stream())
    }
}
