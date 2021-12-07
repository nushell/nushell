use crate::GStat;
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, Signature, Span, Spanned, SyntaxShape, Value};

impl Plugin for GStat {
    fn signature(&self) -> Vec<Signature> {
        vec![Signature::build("gstat")
            .desc("Get the git status of a repo")
            .optional("path", SyntaxShape::String, "path to repo")
            .category(Category::Custom("Prompt".to_string()))]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        if name != "gstat" {
            return Ok(Value::Nothing {
                span: Span::unknown(),
            });
        }

        let repo_path: Option<Spanned<String>> = call.opt(0)?;
        // eprintln!("input value: {:#?}", &input);
        self.gstat(input, repo_path, &call.head)
    }
}
