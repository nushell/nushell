use std::{
    borrow::Cow,
    collections::{HashMap, hash_map},
    sync::Arc,
};

use crate::{BlockId, Record, ShellError, Span, Value, VarId, ast::Block, engine::EngineState};

#[derive(Clone, Debug)]
pub struct Closure {
    pub block_id: BlockId,
    pub captures: Vec<(VarId, Value)>,
    /// Optional inline block for closures that were deserialized and don't have
    /// their block in the engine state. When present, this takes precedence over block_id.
    pub inline_block: Option<Arc<Block>>,
    /// Nested blocks referenced by the inline_block's IR instructions.
    /// These are blocks that would normally be looked up via engine_state.get_block()
    /// but for deserialized closures they are stored here instead.
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

    /// Convert this closure to a Record Value for serialization.
    /// The record includes a type marker so it can be identified during deserialization.
    pub fn to_record(&self, engine_state: &EngineState, span: Span) -> Result<Value, ShellError> {
        let block = self.get_block(engine_state);

        // Convert the block to a nushell Value
        let block_value = block.to_nu_value(span)?;

        // Serialize captures as a list of records with var_id and value.
        // Captured closures are recursively converted to records.
        let mut captures_list: Vec<Value> = Vec::with_capacity(self.captures.len());
        for (var_id, value) in &self.captures {
            let serialized_value = if let Value::Closure { val, .. } = value {
                val.to_record(engine_state, span)?
            } else {
                value.clone()
            };
            captures_list.push(Value::record(
                Record::from_iter([
                    ("var_id".to_string(), Value::int(var_id.get() as i64, span)),
                    ("value".to_string(), serialized_value),
                ]),
                span,
            ));
        }

        // Collect and serialize nested blocks
        let mut nested_blocks_record = Record::new();
        let mut nested_blocks_map: HashMap<usize, &Block> = HashMap::new();

        // Collect nested blocks from the main block
        collect_nested_blocks_recursive(engine_state, block.as_ref(), &mut nested_blocks_map);

        for (block_id, nested_block) in nested_blocks_map {
            let nested_value = nested_block.to_nu_value(span)?;
            nested_blocks_record.push(block_id.to_string(), nested_value);
        }

        let record = Record::from_iter([
            ("block".to_string(), block_value),
            ("captures".to_string(), Value::list(captures_list, span)),
            (
                "nested_blocks".to_string(),
                Value::record(nested_blocks_record, span),
            ),
        ]);

        Ok(Value::record(record, span))
    }

    /// Try to create a Closure from a Record Value.
    /// Returns None if the record is not a serialized closure.
    pub fn from_record(record: &Record, span: Span) -> Result<Option<Self>, ShellError> {
        // Get the block value
        let block_value = match record.get("block") {
            Some(v) => v,
            _ => {
                return Err(ShellError::GenericError {
                    error: "Invalid closure record".into(),
                    msg: "missing or invalid 'block' field".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                });
            }
        };

        // Deserialize the block from the nushell Value
        let block = Block::from_nu_value(block_value)?;

        // Get captures, recursively converting any closure records back to closures
        let captures = match record.get("captures") {
            Some(Value::List { vals, .. }) => {
                let mut captures = Vec::with_capacity(vals.len());
                for val in vals {
                    if let Value::Record { val: rec, .. } = val {
                        let var_id = match rec.get("var_id") {
                            Some(Value::Int { val, .. }) => VarId::new(*val as usize),
                            _ => continue,
                        };
                        let value = match rec.get("value") {
                            Some(Value::Record {
                                val: inner_rec,
                                internal_span,
                            }) if inner_rec.get("block").is_some() => {
                                // This looks like a serialized closure record — convert it back
                                match Closure::from_record(inner_rec, *internal_span)? {
                                    Some(closure) => Value::closure(closure, *internal_span),
                                    None => {
                                        Value::record(inner_rec.as_ref().clone(), *internal_span)
                                    }
                                }
                            }
                            Some(v) => v.clone(),
                            _ => continue,
                        };
                        captures.push((var_id, value));
                    }
                }
                captures
            }
            _ => vec![],
        };

        // Get nested blocks
        let mut nested_blocks: HashMap<BlockId, Arc<Block>> = HashMap::new();
        if let Some(Value::Record {
            val: nested_rec, ..
        }) = record.get("nested_blocks")
        {
            for (key, val) in nested_rec.iter() {
                if let Ok(block_id) = key.parse::<usize>() {
                    let nested_block = Block::from_nu_value(val)?;
                    nested_blocks.insert(BlockId::new(block_id), Arc::new(nested_block));
                }
            }
        }

        Ok(Some(Closure::with_inline_block_and_nested(
            Arc::new(block),
            captures,
            nested_blocks,
        )))
    }
}

/// Collect all BlockIds referenced in a Block's IR instructions
fn collect_block_ids(block: &Block) -> Vec<BlockId> {
    use crate::ir::Literal;

    let mut block_ids = Vec::new();
    if let Some(ref ir_block) = block.ir_block {
        for instruction in &ir_block.instructions {
            if let crate::ir::Instruction::LoadLiteral {
                lit: Literal::Block(id) | Literal::Closure(id) | Literal::RowCondition(id),
                ..
            } = instruction
            {
                block_ids.push(*id);
            }
        }
    }
    block_ids
}

/// Recursively collect all nested blocks from a block
fn collect_nested_blocks_recursive<'a>(
    engine_state: &'a EngineState,
    block: &Block,
    nested_blocks: &mut HashMap<usize, &'a Block>,
) {
    for block_id in collect_block_ids(block) {
        let id_val = block_id.get();
        if let hash_map::Entry::Vacant(e) = nested_blocks.entry(id_val) {
            let nested_block = engine_state.get_block(block_id);
            e.insert(nested_block.as_ref());
            // Recursively collect nested blocks from this block
            collect_nested_blocks_recursive(engine_state, nested_block.as_ref(), nested_blocks);
        }
    }
}
