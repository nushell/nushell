use crate::GStat;
use nu_plugin::{EngineInterface, EvaluatedCall, Plugin, PluginCommand, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, Spanned, SyntaxShape, Value};

pub struct GStatPlugin;

impl Plugin for GStatPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(GStat)]
    }
}

impl SimplePluginCommand for GStat {
    type Plugin = GStatPlugin;

    fn name(&self) -> &str {
        "gstat"
    }

    fn description(&self) -> &str {
        "Get the git status of a repo"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .switch("no-tag", "Disable git tag resolving", None)
            .optional("path", SyntaxShape::Filepath, "path to repo")
            .category(Category::Custom("prompt".to_string()))
    }

    fn run(
        &self,
        _plugin: &GStatPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let repo_path: Option<Spanned<String>> = call.opt(0)?;
        // eprintln!("input value: {:#?}", &input);
        let current_dir = engine.get_current_dir()?;
        let disable_tag = call.has_flag("no-tag")?;

        self.gstat(input, &current_dir, repo_path, !disable_tag, call.head)
    }
}
