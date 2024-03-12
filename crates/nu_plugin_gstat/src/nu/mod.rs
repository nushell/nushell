use crate::GStat;
use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginSignature, Spanned, SyntaxShape, Value};

impl Plugin for GStat {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![PluginSignature::build("gstat")
            .usage("Get the git status of a repo")
            .optional("path", SyntaxShape::Filepath, "path to repo")
            .category(Category::Custom("prompt".to_string()))]
    }

    fn run(
        &self,
        name: &str,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        if name != "gstat" {
            return Ok(Value::nothing(call.head));
        }

        let repo_path: Option<Spanned<String>> = call.opt(0)?;
        // eprintln!("input value: {:#?}", &input);
        let current_dir = engine.get_current_dir()?;
        self.gstat(input, &current_dir, repo_path, call.head)
    }
}
