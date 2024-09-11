use nu_engine::command_prelude::*;
use nu_protocol::{BlockId, DeclId};

#[derive(Clone)]
pub struct ViewIr;

impl Command for ViewIr {
    fn name(&self) -> &str {
        "view ir"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "target",
                SyntaxShape::Any,
                "The name or block to view compiled code for.",
            )
            .switch(
                "json",
                "Dump the raw block data as JSON (unstable).",
                Some('j'),
            )
            .switch(
                "decl-id",
                "Integer is a declaration ID rather than a block ID.",
                Some('d'),
            )
            .input_output_type(Type::Nothing, Type::String)
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        "View the compiled IR code for a block of code."
    }

    fn extra_description(&self) -> &str {
        "
The target can be a closure, the name of a custom command, or an internal block
ID. Closure literals within IR dumps often reference the block by ID (e.g.
`closure(3231)`), so this provides an easy way to read the IR of any embedded
closures.

The --decl-id option is provided to use a declaration ID instead, which can be
found on `call` instructions. This is sometimes better than using the name, as
the declaration may not be in scope.
"
        .trim()
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let target: Value = call.req(engine_state, stack, 0)?;
        let json = call.has_flag(engine_state, stack, "json")?;
        let is_decl_id = call.has_flag(engine_state, stack, "decl-id")?;

        let block_id = match target {
            Value::Closure { ref val, .. } => val.block_id,
            // Decl by name
            Value::String { ref val, .. } => {
                if let Some(decl_id) = engine_state.find_decl(val.as_bytes(), &[]) {
                    let decl = engine_state.get_decl(decl_id);
                    decl.block_id().ok_or_else(|| ShellError::GenericError {
                        error: format!("Can't view IR for `{val}`"),
                        msg: "not a custom command".into(),
                        span: Some(target.span()),
                        help: Some("internal commands don't have Nushell source code".into()),
                        inner: vec![],
                    })?
                } else {
                    return Err(ShellError::GenericError {
                        error: format!("Can't view IR for `{val}`"),
                        msg: "can't find a command with this name".into(),
                        span: Some(target.span()),
                        help: None,
                        inner: vec![],
                    });
                }
            }
            // Decl by ID - IR dump always shows name of decl, but sometimes it isn't in scope
            Value::Int { val, .. } if is_decl_id => {
                let decl_id = val
                    .try_into()
                    .ok()
                    .map(DeclId::new)
                    .filter(|id| id.get() < engine_state.num_decls())
                    .ok_or_else(|| ShellError::IncorrectValue {
                        msg: "not a valid decl id".into(),
                        val_span: target.span(),
                        call_span: call.head,
                    })?;
                let decl = engine_state.get_decl(decl_id);
                decl.block_id().ok_or_else(|| ShellError::GenericError {
                    error: format!("Can't view IR for `{}`", decl.name()),
                    msg: "not a custom command".into(),
                    span: Some(target.span()),
                    help: Some("internal commands don't have Nushell source code".into()),
                    inner: vec![],
                })?
            }
            // Block by ID - often shows up in IR
            Value::Int { val, .. } => {
                val.try_into()
                    .map(BlockId::new)
                    .map_err(|_| ShellError::IncorrectValue {
                        msg: "not a valid block id".into(),
                        val_span: target.span(),
                        call_span: call.head,
                    })?
            }
            // Pass through errors
            Value::Error { error, .. } => return Err(*error),
            _ => {
                return Err(ShellError::TypeMismatch {
                    err_message: "expected closure, string, or int".into(),
                    span: call.head,
                })
            }
        };

        let Some(block) = engine_state.try_get_block(block_id) else {
            return Err(ShellError::GenericError {
                error: format!("Unknown block ID: {}", block_id.get()),
                msg: "ensure the block ID is correct and try again".into(),
                span: Some(target.span()),
                help: None,
                inner: vec![],
            });
        };

        let ir_block = block
            .ir_block
            .as_ref()
            .ok_or_else(|| ShellError::GenericError {
                error: "Can't view IR for this block".into(),
                msg: "block is missing compiled representation".into(),
                span: block.span,
                help: Some("the IrBlock is probably missing due to a compilation error".into()),
                inner: vec![],
            })?;

        let formatted = if json {
            let formatted_instructions = ir_block
                .instructions
                .iter()
                .map(|instruction| {
                    instruction
                        .display(engine_state, &ir_block.data)
                        .to_string()
                })
                .collect::<Vec<_>>();

            serde_json::to_string_pretty(&serde_json::json!({
                "block_id": block_id,
                "span": block.span,
                "ir_block": ir_block,
                "formatted_instructions": formatted_instructions,
            }))
            .map_err(|err| ShellError::GenericError {
                error: "JSON serialization failed".into(),
                msg: err.to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
        } else {
            format!("{}", ir_block.display(engine_state))
        };

        Ok(Value::string(formatted, call.head).into_pipeline_data())
    }
}
