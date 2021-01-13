use crate::prelude::*;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct Alias;

#[derive(Deserialize)]
pub struct AliasArgs {
    pub name: Tagged<String>,
    pub equals: Tagged<String>,
    pub rest: Vec<Tagged<Value>>,
}

#[async_trait]
impl WholeStreamCommand for Alias {
    fn name(&self) -> &str {
        "alias"
    }

    fn signature(&self) -> Signature {
        Signature::build("alias")
            .required("name", SyntaxShape::String, "the name of the command")
            .required("equals", SyntaxShape::String, "the equals sign")
            .rest(SyntaxShape::Any, "the definition for the alias")
    }

    fn usage(&self) -> &str {
        "Create an alias and set it to a definition."
    }

    async fn run(&self, _args: CommandArgs) -> Result<OutputStream, ShellError> {
        // Currently, we don't do anything here because we should have already
        // installed the definition as we entered the scope
        // We just create a command so that we can get proper coloring
        Ok(OutputStream::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}
