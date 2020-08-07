use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use derive_new::new;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, Signature, SyntaxShape};

#[derive(new, Clone)]
pub struct AliasCommand {
    name: String,
    args: Vec<String>,
    block: Block,
}

#[async_trait]
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
        let mut context = Context::from_args(&args, &registry);
        let input = args.input;

        let mut scope = call_info.scope.clone();
        let evaluated = call_info.evaluate(&registry).await?;
        if let Some(positional) = &evaluated.args.positional {
            for (pos, arg) in positional.iter().enumerate() {
                scope
                    .vars
                    .insert(alias_command.args[pos].to_string(), arg.clone());
            }
        }

        // FIXME: we need to patch up the spans to point at the top-level error
        Ok(run_block(
            &block,
            &mut context,
            input,
            &scope.it,
            &scope.vars,
            &scope.env,
        )
        .await?
        .to_output_stream())
    }
}
