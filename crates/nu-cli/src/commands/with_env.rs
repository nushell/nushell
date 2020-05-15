use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;
use parking_lot::Mutex;

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
        with_env(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Set the MYENV environment variable",
            example: r#"with-env [MYENV "my env value"] { echo $nu.env.MYENV }"#,
        }]
    }
}

fn with_env(raw_args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let stream = async_stream! {
        let mut context;
        #[cfg(windows)]
        {
            context = Context {
                registry: registry.clone(),
                host: raw_args.host.clone(),
                current_errors: Arc::new(Mutex::new(vec![])),
                ctrl_c: raw_args.ctrl_c.clone(),
                shell_manager: raw_args.shell_manager.clone(),
                windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
            };
        }
        #[cfg(not(windows))]
        {
            context = Context {
                registry: registry.clone(),
                host: raw_args.host.clone(),
                current_errors: Arc::new(Mutex::new(vec![])),
                ctrl_c: raw_args.ctrl_c.clone(),
                shell_manager: raw_args.shell_manager.clone(),
            };
        }

        let scope = raw_args
            .call_info
            .scope
            .clone();
        let (WithEnvArgs { variable, block }, mut input) = raw_args.process(&registry).await?;
        let scope = scope.set_env_var(variable.0.item, variable.1.item);

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
