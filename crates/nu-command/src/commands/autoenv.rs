use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
pub struct Autoenv;

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Trusted {
    pub files: IndexMap<String, Vec<u8>>,
}
impl Trusted {
    pub fn new() -> Self {
        Trusted {
            files: IndexMap::new(),
        }
    }
}
pub fn file_is_trusted(nu_env_file: &Path, content: &[u8]) -> Result<bool, ShellError> {
    let contentdigest = Sha256::digest(&content).as_slice().to_vec();
    let nufile = std::fs::canonicalize(nu_env_file)?;

    let trusted = read_trusted()?;

    Ok(trusted.files.get(&nufile.to_string_lossy().to_string()) == Some(&contentdigest))
}

pub fn read_trusted() -> Result<Trusted, ShellError> {
    let config_path = config::default_path_for(&Some(PathBuf::from("nu-env.toml")))?;

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .create(true)
        .write(true)
        .open(config_path)
        .map_err(|_| ShellError::untagged_runtime_error("Couldn't open nu-env.toml"))?;
    let mut doc = String::new();
    file.read_to_string(&mut doc)?;

    let allowed = toml::de::from_str(doc.as_str()).unwrap_or_else(|_| Trusted::new());
    Ok(allowed)
}

#[async_trait]
impl WholeStreamCommand for Autoenv {
    fn name(&self) -> &str {
        "autoenv"
    }
    fn usage(&self) -> &str {
        // "Mark a .nu-env file in a directory as trusted. Needs to be re-run after each change to the file or its filepath."
        r#"Manage directory specific environment variables and scripts. Create a file called .nu-env in any directory and run 'autoenv trust' to let nushell read it when entering the directory.
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
            UntaggedValue::string(get_help(&Autoenv, &args.scope)).into_value(Tag::unknown()),
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
