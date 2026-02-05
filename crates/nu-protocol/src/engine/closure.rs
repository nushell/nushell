use std::{borrow::Cow, collections::HashMap, sync::Arc};

use crate::{BlockId, ShellError, Span, Value, VarId, ast::Block, engine::EngineState};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Closure {
    pub block_id: BlockId,
    pub captures: Vec<(VarId, Value)>,
    /// Optional inline block for closures that were deserialized and don't have
    /// their block in the engine state. When present, this takes precedence over block_id.
    #[serde(skip)]
    pub inline_block: Option<Arc<Block>>,
    /// Nested blocks referenced by the inline_block's IR instructions.
    /// These are blocks that would normally be looked up via engine_state.get_block()
    /// but for deserialized closures they are stored here instead.
    #[serde(skip)]
    pub nested_blocks: HashMap<BlockId, Arc<Block>>,
}

impl Closure {
    /// Create a new closure with just a block_id and captures
    pub fn new(block_id: BlockId, captures: Vec<(VarId, Value)>) -> Self {
        Self {
            block_id,
            captures,
            inline_block: None,
            nested_blocks: HashMap::new(),
        }
    }

    /// Create a new closure with an inline block (for deserialized closures)
    pub fn with_inline_block(block: Arc<Block>, captures: Vec<(VarId, Value)>) -> Self {
        Self {
            block_id: BlockId::new(0), // Placeholder, inline_block takes precedence
            captures,
            inline_block: Some(block),
            nested_blocks: HashMap::new(),
        }
    }

    /// Create a new closure with an inline block and nested blocks (for deserialized closures)
    pub fn with_inline_block_and_nested(
        block: Arc<Block>,
        captures: Vec<(VarId, Value)>,
        nested_blocks: HashMap<BlockId, Arc<Block>>,
    ) -> Self {
        Self {
            block_id: BlockId::new(0), // Placeholder, inline_block takes precedence
            captures,
            inline_block: Some(block),
            nested_blocks,
        }
    }

    /// Get the block, either from inline_block or from engine_state
    pub fn get_block<'a>(&'a self, engine_state: &'a EngineState) -> &'a Arc<Block> {
        if let Some(ref block) = self.inline_block {
            // For inline blocks, we need to return a reference with the right lifetime
            // This is safe because self lives as long as 'a
            block
        } else {
            engine_state.get_block(self.block_id)
        }
    }

    /// Get a nested block by ID, checking inline nested_blocks first, then engine_state
    pub fn get_nested_block<'a>(
        &'a self,
        block_id: BlockId,
        engine_state: &'a EngineState,
    ) -> &'a Arc<Block> {
        if let Some(block) = self.nested_blocks.get(&block_id) {
            block
        } else {
            engine_state.get_block(block_id)
        }
    }

    /// Check if this closure has any nested blocks
    pub fn has_nested_blocks(&self) -> bool {
        !self.nested_blocks.is_empty()
    }

    pub fn coerce_into_string<'a>(
        &self,
        engine_state: &'a EngineState,
        span: Span,
    ) -> Result<Cow<'a, str>, ShellError> {
        let block = self.get_block(engine_state);
        if let Some(block_span) = block.span {
            let contents_bytes = engine_state.get_span_contents(block_span);
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
}
