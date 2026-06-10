use std::{
    borrow::Cow,
    fmt::{self, Debug},
};

use crate::{BlockId, ShellError, Span, Value, VarId, engine::EngineState};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Closure {
    pub block_id: BlockId,
    pub captures: Vec<(VarId, Value)>,
}

impl Debug for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Closure")
            .field("block_id", &self.block_id)
            .field(
                "captures",
                &fmt::from_fn(|f| {
                    f.debug_map()
                        .entries(self.captures.iter().map(|(k, v)| (k, v)))
                        .finish()
                }),
            )
            .finish()
    }
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

    /// Returns an estimate of the memory size used by this Closure in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self
                .captures
                .iter()
                .map(|(_, v)| v.memory_size())
                .sum::<usize>()
    }

    pub(crate) fn compact_debug(&self) -> impl Debug {
        fmt::from_fn(|f| {
            write!(f, "{:?}: ", self.block_id)?;
            f.debug_map()
                .entries(self.captures.iter().map(|(k, v)| (k, v)))
                .finish()
        })
    }
}
