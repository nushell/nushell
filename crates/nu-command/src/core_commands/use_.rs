use nu_engine::eval_block;
use nu_protocol::ast::{Call, Expr, Expression, ImportPatternMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct Use;

impl Command for Use {
    fn name(&self) -> &str {
        "use"
    }

    fn usage(&self) -> &str {
        "Use definitions from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("use")
            .required("pattern", SyntaxShape::ImportPattern, "import pattern")
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let import_pattern = if let Some(Expression {
            expr: Expr::ImportPattern(pat),
            ..
        }) = call.positional.get(0)
        {
            pat
        } else {
            return Err(ShellError::SpannedLabeledError(
                "Unexpected import".into(),
                "import pattern not supported".into(),
                call.head,
            ));
        };

        if let Some(overlay_id) = engine_state.find_overlay(&import_pattern.head.name) {
            let overlay = engine_state.get_overlay(overlay_id);

            let env_vars_to_use = if import_pattern.members.is_empty() {
                overlay.env_vars_with_head(&import_pattern.head.name)
            } else {
                match &import_pattern.members[0] {
                    ImportPatternMember::Glob { .. } => overlay.env_vars(),
                    ImportPatternMember::Name { name, span } => {
                        let mut output = vec![];

                        if let Some(id) = overlay.get_env_var_id(name) {
                            output.push((name.clone(), id));
                        } else if !overlay.has_decl(name) {
                            return Err(ShellError::EnvVarNotFoundAtRuntime(
                                String::from_utf8_lossy(name).into(),
                                *span,
                            ));
                        }

                        output
                    }
                    ImportPatternMember::List { names } => {
                        let mut output = vec![];

                        for (name, span) in names {
                            if let Some(id) = overlay.get_env_var_id(name) {
                                output.push((name.clone(), id));
                            } else if !overlay.has_decl(name) {
                                return Err(ShellError::EnvVarNotFoundAtRuntime(
                                    String::from_utf8_lossy(name).into(),
                                    *span,
                                ));
                            }
                        }

                        output
                    }
                }
            };

            for (name, block_id) in env_vars_to_use {
                let name = if let Ok(s) = String::from_utf8(name.clone()) {
                    s
                } else {
                    return Err(ShellError::NonUtf8(import_pattern.head.span));
                };

                let block = engine_state.get_block(block_id);

                // TODO: Add string conversions (e.g. int to string)
                // TODO: Later expand env to take all Values
                let val = eval_block(engine_state, stack, block, PipelineData::new(call.head))?
                    .into_value(call.head);

                stack.add_env_var(name, val);
            }
        } else {
            // TODO: This is a workaround since call.positional[0].span points at 0 for some reason
            // when this error is triggered
            let bytes = engine_state.get_span_contents(&call.positional[0].span);
            return Err(ShellError::SpannedLabeledError(
                format!(
                    "Could not use '{}' import pattern",
                    String::from_utf8_lossy(bytes)
                ),
                "called here".to_string(),
                call.head,
            ));
        }

        Ok(PipelineData::new(call.head))
    }
}
