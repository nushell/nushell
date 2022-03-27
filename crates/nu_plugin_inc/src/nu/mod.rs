use crate::inc::SemVerAction;
use crate::Inc;
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{ast::CellPath, Signature, SyntaxShape, Value};

impl Plugin for Inc {
    fn signature(&self) -> Vec<Signature> {
        vec![Signature::build("inc")
            .usage("Increment a value or version. Optionally use the column of a table.")
            .optional("cell_path", SyntaxShape::CellPath, "cell path to update")
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

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        if name != "inc" {
            return Ok(Value::Nothing { span: call.head });
        }

        let cell_path: Option<CellPath> = call.opt(0)?;

        self.cell_path = cell_path;

        if call.has_flag("major") {
            self.for_semver(SemVerAction::Major);
        }
        if call.has_flag("minor") {
            self.for_semver(SemVerAction::Minor);
        }
        if call.has_flag("patch") {
            self.for_semver(SemVerAction::Patch);
        }

        self.inc(call.head, input)
    }
}
