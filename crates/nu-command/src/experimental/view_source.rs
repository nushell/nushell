use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct ViewSource;

impl Command for ViewSource {
    fn name(&self) -> &str {
        "view-source"
    }

    fn usage(&self) -> &str {
        "View a block, module, or a definition"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view-source")
            .desc(self.usage())
            .required("item", SyntaxShape::Any, "name or block to view")
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let arg: Value = call.req(engine_state, stack, 0)?;
        let arg_span = arg.span()?;

        match arg {
            Value::Block { span, .. } => {
                let contents = engine_state.get_span_contents(&span);
                Ok(
                    Value::string(String::from_utf8_lossy(contents), call.head)
                        .into_pipeline_data(),
                )
            }
            Value::String { val, .. } => {
                if let Some(decl_id) = engine_state.find_decl(val.as_bytes()) {
                    // arg is a command
                    let decl = engine_state.get_decl(decl_id);
                    if let Some(block_id) = decl.get_block_id() {
                        let block = engine_state.get_block(block_id);
                        if let Some(block_span) = block.span {
                            let contents = engine_state.get_span_contents(&block_span);
                            Ok(Value::string(String::from_utf8_lossy(contents), call.head)
                                .into_pipeline_data())
                        } else {
                            Err(ShellError::SpannedLabeledError(
                                "Cannot view value".to_string(),
                                "the command does not have a viewable block".to_string(),
                                arg_span,
                            ))
                        }
                    } else {
                        Err(ShellError::SpannedLabeledError(
                            "Cannot view value".to_string(),
                            "the command does not have a viewable block".to_string(),
                            arg_span,
                        ))
                    }
                } else if let Some(overlay_id) = engine_state.find_overlay(val.as_bytes()) {
                    // arg is a module
                    let overlay = engine_state.get_overlay(overlay_id);
                    if let Some(overlay_span) = overlay.span {
                        let contents = engine_state.get_span_contents(&overlay_span);
                        Ok(Value::string(String::from_utf8_lossy(contents), call.head)
                            .into_pipeline_data())
                    } else {
                        Err(ShellError::SpannedLabeledError(
                            "Cannot view value".to_string(),
                            "the module does not have a viewable block".to_string(),
                            arg_span,
                        ))
                    }
                } else {
                    Err(ShellError::SpannedLabeledError(
                        "Cannot view value".to_string(),
                        "this name does not correspond to a viewable value".to_string(),
                        arg_span,
                    ))
                }
            }
            _ => Err(ShellError::SpannedLabeledError(
                "Cannot view value".to_string(),
                "this value cannot be viewed".to_string(),
                arg_span,
            )),
        }
    }
}
