use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    IntoSpanned, LabeledError, PipelineData, Record, Signature, Spanned, SyntaxShape, Value,
};

use crate::ExamplePlugin;

pub struct CallDecl;

impl PluginCommand for CallDecl {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example call-decl"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "name",
                SyntaxShape::String,
                "the name of the command to call",
            )
            .optional(
                "named_args",
                SyntaxShape::Record(vec![]),
                "named arguments to pass to the command",
            )
            .rest(
                "positional_args",
                SyntaxShape::Any,
                "positional arguments to pass to the command",
            )
    }

    fn description(&self) -> &str {
        "Demonstrates calling other commands from plugins using `call_decl()`."
    }

    fn extra_description(&self) -> &str {
        "
The arguments will not be typechecked at parse time. This command is for
demonstration only, and should not be used for anything real.
"
        .trim()
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let name: Spanned<String> = call.req(0)?;
        let named_args: Option<Record> = call.opt(1)?;
        let positional_args: Vec<Value> = call.rest(2)?;

        let decl_id = engine.find_decl(&name.item)?.ok_or_else(|| {
            LabeledError::new(format!("Can't find `{}`", name.item))
                .with_label("not in scope", name.span)
        })?;

        let mut new_call = EvaluatedCall::new(call.head);

        for (key, val) in named_args.into_iter().flatten() {
            new_call.add_named(key.into_spanned(val.span()), val);
        }

        for val in positional_args {
            new_call.add_positional(val);
        }

        let result = engine.call_decl(decl_id, new_call, input, true, false)?;

        Ok(result)
    }
}
