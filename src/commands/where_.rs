use crate::commands::PerItemCommand;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry;
use crate::prelude::*;

pub struct Where;

impl PerItemCommand for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn signature(&self) -> registry::Signature {
        Signature::build("where").required("condition", SyntaxType::Block)
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &registry::CommandRegistry,
        _shell_manager: &ShellManager,
        input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        let input_clone = input.clone();
        let condition = call_info.args.expect_nth(0)?;
        let stream = match condition {
            Tagged {
                item: Value::Block(block),
                tag,
            } => {
                let result = block.invoke(&input_clone);
                match result {
                    Ok(v) => {
                        if v.is_true() {
                            VecDeque::from(vec![Ok(ReturnSuccess::Value(input_clone))])
                        } else {
                            VecDeque::new()
                        }
                    }
                    Err(e) => {
                        return Err(ShellError::labeled_error(
                            format!("Could not evaluate ({})", e.to_string()),
                            "could not evaluate",
                            tag.span,
                        ))
                    }
                }
            }
            Tagged { tag, .. } => {
                return Err(ShellError::labeled_error(
                    "Expected a condition",
                    "where needs a condition",
                    tag.span,
                ))
            }
        };

        Ok(stream.to_output_stream())
    }
}
