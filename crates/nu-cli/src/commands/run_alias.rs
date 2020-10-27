use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use derive_new::new;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, PositionalType, Scope, Signature, UntaggedValue};

#[derive(new, Clone)]
pub struct AliasCommand {
    sig: Signature,
    block: Block,
}

#[async_trait]
impl WholeStreamCommand for AliasCommand {
    fn name(&self) -> &str {
        &self.sig.name
    }

    fn signature(&self) -> Signature {
        self.sig.clone()
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

        // let alias_command = self.clone();
        let mut context = EvaluationContext::from_args(&args, &registry);
        let input = args.input;

        let scope = call_info.scope.clone();
        let evaluated = call_info.evaluate(&registry).await?;

        let mut vars = IndexMap::new();
        let mut num_positionals = 0;
        if let Some(positional) = &evaluated.args.positional {
            num_positionals = positional.len();
            for (idx, arg) in positional.iter().enumerate() {
                let pos_type = &self.sig.positional[idx].0;
                match pos_type {
                    PositionalType::Mandatory(name, _) | PositionalType::Optional(name, _) => {
                        vars.insert(name.clone(), arg.clone());
                    }
                }
            }
        }
        //Fill out every missing argument with empty value
        if self.sig.positional.len() > num_positionals {
            for idx in num_positionals..self.sig.positional.len() {
                let pos_type = &self.sig.positional[idx].0;
                match pos_type {
                    PositionalType::Mandatory(name, _) | PositionalType::Optional(name, _) => {
                        vars.insert(name.clone(), UntaggedValue::nothing().into_untagged_value());
                    }
                }
            }
        }

        let scope = Scope::append_vars(scope, vars);

        // FIXME: we need to patch up the spans to point at the top-level error
        Ok(run_block(&block, &mut context, input, scope)
            .await?
            .to_output_stream())
    }
}
