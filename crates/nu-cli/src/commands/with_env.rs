use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct WithEnv;

#[derive(Deserialize, Debug)]
struct WithEnvArgs {
    variable: (Tagged<String>, Tagged<String>),
    block: Block,
}
impl WholeStreamCommand for WithEnv {
    fn name(&self) -> &str {
        "with-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("with-env")
            .required(
                "variable",
                SyntaxShape::Any,
                "the environment variable to temporarily set",
            )
            .required(
                "block",
                SyntaxShape::Block,
                "the block to run once the variable is set",
            )
    }

    fn usage(&self) -> &str {
        "Runs a block with an environment set. Eg) with-env [NAME 'foo'] { echo $nu.env.NAME }"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(args.process_raw(registry, with_env)?.run())
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Set the MYENV environment variable",
            example: r#"with-env [MYENV "my env value"] { echo $nu.env.MYENV }"#,
        }]
    }
}

fn with_env(
    WithEnvArgs { variable, block }: WithEnvArgs,
    context: RunnableContext,
    raw_args: RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let scope = raw_args
        .call_info
        .scope
        .clone()
        .set_env_var(variable.0.item, variable.1.item);
    let registry = context.registry.clone();
    let input = context.input;
    let mut context = Context::from_raw(&raw_args, &registry);

    let stream = async_stream! {
        let result = run_block(
            &block,
            &mut context,
            input,
            &scope.clone(),
        ).await;

        match result {
            Ok(mut stream) => {
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
