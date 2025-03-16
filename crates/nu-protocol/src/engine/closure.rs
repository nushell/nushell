use std::borrow::Cow;

use crate::{engine::EngineState, BlockId, ShellError, Span, Value, VarId};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Closure {
    pub block_id: BlockId,
    pub captures: Vec<(VarId, Value)>,
}

impl Closure {
    pub fn coerce_into_string<'a>(
        &self,
        engine_state: &'a EngineState,
        span: Span,
    ) -> Result<Cow<'a, str>, ShellError> {
        let block = engine_state.get_block(self.block_id);
        if let Some(span) = block.span {
            let contents_bytes = engine_state.get_span_contents(span);
            Ok(String::from_utf8_lossy(contents_bytes))
        } else {
            Err(ShellError::CantConvert {
                to_type: "string".into(),
                from_type: "closure".into(),
                span,
                help: Some(format!(
                    "unable to retrieve block contents for closure with id {}",
                    self.block_id.get()
                )),
            })
        }
    }
}
