use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use derive_new::new;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, PositionalType, Scope, Signature, UntaggedValue, Value};

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

        if let Some(positional) = &evaluated.args.positional {
            for (idx, (pos_type, _)) in self.sig.positional.iter().enumerate() {
                let arg = &positional[idx];
                match pos_type {
                    PositionalType::Mandatory(name, _) | PositionalType::Optional(name, _) => {
                        scope.vars.insert(name.clone(), arg.clone());
                    }
                }
            }
            if let Some((_, desc)) = &self.sig.rest_positional {
                let var_arg_idx = self.sig.positional.len();
                if var_arg_idx < positional.len() {
                    let var_arg_val = Value {
                        value: UntaggedValue::Table(positional[var_arg_idx..].to_vec()),
                        tag: positional[var_arg_idx]
                            .tag
                            .until(&positional.last().unwrap_or(&Value::nothing()).tag),
                    };
                    trace!("Inserting for var arg: {:?}", var_arg_val);
                    //For now description contains name. This is a little hacky :(
                    let name = desc.split(": ").next().unwrap_or("$args");
                    scope.vars.insert(name.to_string(), var_arg_val);
                }
            }
        }

        let scope = Scope::append_vars(scope, vars);

        // FIXME: we need to patch up the spans to point at the top-level error
        Ok(run_block(&block, &mut context, input, scope)
            .await?
            .to_output_stream())
    }
    fn is_binary(&self) -> bool {
        false
    }
    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }
}
