use crate::{Inc, inc::SemVerAction};
use nu_plugin::{EngineInterface, EvaluatedCall, Plugin, PluginCommand, SimplePluginCommand};
use nu_protocol::{LabeledError, Signature, SyntaxShape, Value, ast::CellPath};

pub struct IncPlugin;

impl Plugin for IncPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Inc::new())]
    }
}

impl SimplePluginCommand for Inc {
    type Plugin = IncPlugin;

    fn name(&self) -> &str {
        "inc"
    }

    fn description(&self) -> &str {
        "Increment a value or version. Optionally use the column of a table."
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .optional("cell_path", SyntaxShape::CellPath, "Cell path to update.")
            .switch(
                "major",
                "Increment the major version (eg 1.2.1 -> 2.0.0).",
                Some('M'),
            )
            .switch(
                "minor",
                "Increment the minor version (eg 1.2.1 -> 1.3.0).",
                Some('m'),
            )
            .switch(
                "patch",
                "Increment the patch version (eg 1.2.1 -> 1.2.2).",
                Some('p'),
            )
    }

    fn run(
        &self,
        _plugin: &IncPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let mut inc = self.clone();

        let cell_path: Option<CellPath> = call.opt(0)?;

        inc.cell_path = cell_path;

        if call.has_flag("major")? {
            inc.for_semver(SemVerAction::Major);
        }
        if call.has_flag("minor")? {
            inc.for_semver(SemVerAction::Minor);
        }
        if call.has_flag("patch")? {
            inc.for_semver(SemVerAction::Patch);
        }

        inc.inc(call.head, input)
    }
}
