use crate::GStat;
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, Signature, Spanned, SyntaxShape, Value};

impl Plugin for GStat {
    fn signature(&self) -> Vec<Signature> {
        vec![Signature::build("gstat")
            .usage("Get the git status of a repo")
            .optional("path", SyntaxShape::Filepath, "path to repo")
            .category(Category::Custom("prompt".to_string()))]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        if name != "gstat" {
            return Ok(Value::Nothing { span: call.head });
        }

        let repo_path: Option<Spanned<String>> = call.opt(0)?;
        // eprintln!("input value: {:#?}", &input);
        self.gstat(input, repo_path, &call.head)
    }
}
