use crate::inc::SemVerAction;
use crate::Inc;
use nu_plugin::{plugin::PluginError, Plugin};
use nu_protocol::ast::Call;
use nu_protocol::{Signature, Span, Value};

impl Plugin for Inc {
    fn signature(&self) -> Vec<Signature> {
        vec![Signature::build("inc")
            .desc("Increment a value or version. Optionally use the column of a table.")
            .switch(
                "major",
                "increment the major version (eg 1.2.1 -> 2.0.0)",
                Some('M'),
            )
            .switch(
                "minor",
                "increment the minor version (eg 1.2.1 -> 1.3.0)",
                Some('m'),
            )
            .switch(
                "patch",
                "increment the patch version (eg 1.2.1 -> 1.2.2)",
                Some('p'),
            )]
    }

    fn run(&mut self, name: &str, call: &Call, input: &Value) -> Result<Value, PluginError> {
        if name != "inc" {
            return Ok(Value::Nothing {
                span: Span::unknown(),
            });
        }

        if call.has_flag("major") {
            self.for_semver(SemVerAction::Major);
        }
        if call.has_flag("minor") {
            self.for_semver(SemVerAction::Minor);
        }
        if call.has_flag("patch") {
            self.for_semver(SemVerAction::Patch);
        }

        self.inc(input)
    }
}
