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
impl AliasCommand {
    fn assign_values_to_variables(&self, positional: &[Value]) -> IndexMap<String, Value> {
        let mut vars = IndexMap::new();
        let nothing = vec![UntaggedValue::nothing().into_untagged_value()];
        self.sig
            .positional
            .iter()
            .zip(positional.iter().chain(nothing.iter().cycle()))
            .for_each(|((pos_type, _), arg)| match pos_type {
                PositionalType::Mandatory(name, _) | PositionalType::Optional(name, _) => {
                    vars.insert(name.clone(), arg.clone());
                }
            });

        if let Some((_, desc)) = &self.sig.rest_positional {
            let var_arg_idx = self.sig.positional.len();
            let var_arg_val = if var_arg_idx < positional.len() {
                let values = positional[var_arg_idx..].to_vec();
                Value {
                    value: UntaggedValue::Table(values),
                    tag: positional[var_arg_idx]
                        .tag
                        .until(&positional.last().unwrap_or(&Value::nothing()).tag),
                }
            } else {
                //Fill missing vararg with empty value
                UntaggedValue::table(&[]).into_untagged_value()
            };
            //For now description contains name. This is a little hacky :(
            let name = desc.split(": ").next().unwrap_or("$args");
            trace!("Inserting for var arg: {:?} value: {:?}", name, var_arg_val);
            vars.insert(name.to_string(), var_arg_val);
        }

        vars
    }

    fn assign_nothing_to_variables(&self) -> IndexMap<String, Value> {
        let mut vars = IndexMap::new();
        for (pos_type, _) in self.sig.positional.iter() {
            match pos_type {
                PositionalType::Mandatory(name, _) | PositionalType::Optional(name, _) => {
                    vars.insert(name.clone(), UntaggedValue::nothing().into_untagged_value());
                }
            }
        }

        if let Some((_, desc)) = &self.sig.rest_positional {
            //For now description contains name. This is a little hacky :(
            let name = desc.split(": ").next().unwrap_or("$args");
            trace!("Inserting for var arg: {:?} value: Nothing", name);
            vars.insert(
                name.to_string(),
                UntaggedValue::table(&[]).into_untagged_value(),
            );
        }

        vars
    }
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

        let vars = if let Some(positional) = &evaluated.args.positional {
            self.assign_values_to_variables(positional)
        } else {
            self.assign_nothing_to_variables()
        };

        let scope = Scope::append_vars(scope, vars);

        // FIXME: we need to patch up the spans to point at the top-level error
        Ok(run_block(&block, &mut context, input, scope)
            .await?
            .to_output_stream())
    }
}
