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
        r#"Create a file called .nu-env in any directory and run 'autoenv trust' to let nushell read it when entering the directory.
The file can contain several optional sections:
    env: environment variables to set when visiting the directory. The variables are unset after leaving the directory and any overwritten values are restored.
    scriptvars: environment variables that should be set to the return value of a script. After they have been set, they behave in the same way as variables set in the env section.
    scripts: scripts to run when entering the directory or leaving it."#
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
        [env]
        mykey = "myvalue"

        [scriptvars]
        myscript = "echo myval"

        [scripts]
        entryscripts = ["touch hello.txt", "touch hello2.txt"]
        exitscripts = ["touch bye.txt"]"#,
            result: None,
        }]
    }
}
