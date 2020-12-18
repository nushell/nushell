use crate::prelude::*;
use crate::{commands::WholeStreamCommand, evaluate::evaluate_baseline_expr};

use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock,
    hir::{ClassifiedCommand, Operator},
    CommandAction, ReturnSuccess, Signature, SyntaxShape, Value,
};
use nu_source::Tagged;

pub struct Def;

#[derive(Deserialize)]
pub struct DefArgs {
    pub name: Tagged<String>,
    pub args: Tagged<Vec<Value>>,
    pub block: CapturedBlock,
}

#[async_trait]
impl WholeStreamCommand for Def {
    fn name(&self) -> &str {
        "def"
    }

    fn signature(&self) -> Signature {
        Signature::build("def")
            .required("name", SyntaxShape::String, "the name of the command")
            .required(
                "params",
                SyntaxShape::Table,
                "the parameters of the command",
            )
            .required("block", SyntaxShape::Block, "the body of the command")
    }

    fn usage(&self) -> &str {
        "Create a command and set it to a definition."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        // Currently, we don't do anything here because we should have already
        // installed the definition as we entered the scope
        // We just create a command so that we can get proper coloring
        Ok(OutputStream::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}
