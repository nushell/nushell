use crate::commands::{PerItemCommand, RawCommandArgs};
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
        args: RawCommandArgs,
        registry: &registry::CommandRegistry,
        input: Tagged<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        let input_clone = input.clone();
        let call_info = args
            .with_input(vec![input])
            .evaluate_once(registry)
            .unwrap();
        let condition = &call_info.args.call_info.args.positional.unwrap()[0];
        match condition {
            Tagged {
                item: Value::Block(block),
                tag,
            } => {
                let result = block.invoke(&input_clone);
                match result {
                    Ok(v) => {
                        if v.is_true() {
                            Ok(VecDeque::from(vec![Ok(ReturnSuccess::Value(input_clone))]))
                        } else {
                            Ok(VecDeque::new())
                        }
                    }
                    Err(e) => Err(ShellError::labeled_error(
                        format!("Could not evaluate ({})", e.to_string()),
                        "could not evaluate",
                        tag.span,
                    )),
                }
            }
            Tagged { tag, .. } => Err(ShellError::labeled_error(
                "Expected a condition",
                "where needs a condition",
                tag.span,
            )),
        }
    }
}
