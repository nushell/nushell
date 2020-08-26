use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;


use derive_new::new;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, Signature, Value, UntaggedValue, VarShapeDeduction, VarDeclaration, SyntaxShape};

#[derive(new, Clone)]
pub struct AliasCommand {
    name: String,
    args: Vec<VarShapeDeduction>,
    block: Block,
}

#[async_trait]
impl WholeStreamCommand for AliasCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        let mut alias = Signature::build(&self.name);

        for (arg, deducted_shape) in self.args.iter().take_while(|arg| !arg.0.is_var_arg) {
            let shape = match deducted_shape{
                //TODO allow signatures with multiple versions. For now we pick first
                VarSyntaxShapeInference::OneOf(options) => {
                    let default = (SyntaxShape::Any, Span::default());
                    options.first().unwrap_or(&default).0
                }
                VarSyntaxShapeInference::MultipleOneOf(_, _) => {unreachable!()}
            };
            //TODO add "deducted by span" as explanation?
            alias = alias.required(arg.name.clone(), shape, "");
        }

        //If we have an var arg
        //TODO add var arg
        // if let Some((arg, shape)) = self.args.last() {
        //     if is_var_arg(arg){
        //         //Added above to positionals, move it to rest
        //         alias.positional.pop();
        //         alias = alias.rest(*shape, arg);
        //     }
        // }

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
            let mut normal_args = alias_command.args.len();
            if let Some((arg, _)) = alias_command.args.last(){
                if arg.is_var_arg{
                    normal_args = normal_args - 1;

                    if positional.len() <= normal_args{
                        //If var arg is empty list
                        //TODO make it to UntaggedValue::VarArg
                        scope.vars.insert(arg.name.clone(), UntaggedValue::Table(Vec::new()).into_untagged_value());
                    }else{
                        let var_arg_idx = alias_command.args.len() - 1;
                        let var_arg_val = Value{
                            value: UntaggedValue::Table(positional[var_arg_idx..].to_vec()),
                            tag: positional[var_arg_idx].tag.until(&positional.last().unwrap().tag),
                        };
                        scope.vars.insert(arg.name.clone(), var_arg_val);
                    }
                }
            }

            for (pos, arg) in positional.iter().enumerate().take(normal_args) {
                scope
                    .vars
                    .insert(alias_command.args[pos].0.name.clone(), arg.clone());
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
    fn is_binary(&self) -> bool {
        false
    }
    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }
}
