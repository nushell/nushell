use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
pub struct Autoenv;

#[async_trait]
impl WholeStreamCommand for Autoenv {
    fn name(&self) -> &str {
        "autoenv"
    }
    fn usage(&self) -> &str {
        "Manage directory specific environment variables and scripts."
    }

    fn extra_usage(&self) -> &str {
        // "Mark a .nu-env file in a directory as trusted. Needs to be re-run after each change to the file or its filepath."
        r#"Create a file called .nu-env in any directory and run 'autoenv trust' to let nushell load it when entering the directory.
The .nu-env file has the same format as your $HOME/nu/config.toml file. By loading a .nu-env file the following applies:
    - environment variables (section \"[env]\") are loaded from the .nu-env file. Those env variables only exist in this directory (and children directories)
    - the \"startup\" commands are run when entering the directory
    - the \"on_exit\" commands are run when leaving the directory
"#
    }

    fn signature(&self) -> Signature {
        Signature::build("autoenv")
    }
    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(get_full_help(&Autoenv, &args.scope)).into_value(Tag::unknown()),
        )))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Example .nu-env file",
            example: r#"cat .nu-env
        startup = ["echo ...entering the directory", "echo 1 2 3"]
        on_exit = ["echo ...leaving the directory"]

        [env]
        mykey = "myvalue"
            "#,
            result: None,
        }]
    }
}
