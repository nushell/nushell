use crate::GStat;
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginSignature, Spanned, SpannedValue, SyntaxShape};

impl Plugin for GStat {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![PluginSignature::build("gstat")
            .usage("Get the git status of a repo")
            .optional("path", SyntaxShape::Filepath, "path to repo")
            .category(Category::Custom("prompt".to_string()))]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &SpannedValue,
    ) -> Result<SpannedValue, LabeledError> {
        if name != "gstat" {
            return Ok(SpannedValue::Nothing { span: call.head });
        }

        let repo_path: Option<Spanned<String>> = call.opt(0)?;
        // eprintln!("input value: {:#?}", &input);
        self.gstat(input, repo_path, call.head)
    }
}
