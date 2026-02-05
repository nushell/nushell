use super::Pipeline;
use crate::{
    OutDest, Record, ShellError, Signature, Span, Type, Value, VarId, engine::StateWorkingSet,
    ir::IrBlock,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub signature: Box<Signature>,
    pub pipelines: Vec<Pipeline>,
    pub captures: Vec<(VarId, Span)>,
    pub redirect_env: bool,
    /// The block compiled to IR instructions. Not available for subexpressions.
    pub ir_block: Option<IrBlock>,
    pub span: Option<Span>, // None option encodes no span to avoid using test_span()
}

impl Block {
    pub fn len(&self) -> usize {
        self.pipelines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }

    pub fn pipe_redirection(
        &self,
        working_set: &StateWorkingSet,
    ) -> (Option<OutDest>, Option<OutDest>) {
        if let Some(first) = self.pipelines.first() {
            first.pipe_redirection(working_set)
        } else {
            (None, None)
        }
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Block {
    pub fn new() -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            pipelines: vec![],
            captures: vec![],
            redirect_env: false,
            ir_block: None,
            span: None,
        }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            pipelines: Vec::with_capacity(capacity),
            captures: vec![],
            redirect_env: false,
            ir_block: None,
            span: None,
        }
    }

    pub fn output_type(&self) -> Type {
        if let Some(last) = self.pipelines.last() {
            if let Some(last) = last.elements.last() {
                if last.redirection.is_some() {
                    Type::Any
                } else {
                    last.expr.ty.clone()
                }
            } else {
                Type::Nothing
            }
        } else {
            Type::Nothing
        }
    }

    /// Replace any `$in` variables in the initial element of pipelines within the block
    pub fn replace_in_variable(
        &mut self,
        working_set: &mut StateWorkingSet<'_>,
        new_var_id: VarId,
    ) {
        for pipeline in self.pipelines.iter_mut() {
            if let Some(element) = pipeline.elements.first_mut() {
                element.replace_in_variable(working_set, new_var_id);
            }
        }
    }
}

impl Block {
    /// Convert this block to a nushell Value (record) for serialization.
    /// Uses serde_json internally to convert the Block's structure into a
    /// tree of nushell values (records, lists, ints, strings, etc.).
    pub fn to_nu_value(&self, span: Span) -> Result<Value, ShellError> {
        let json_value = serde_json::to_value(self).map_err(|e| ShellError::GenericError {
            error: "Failed to serialize block".into(),
            msg: e.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;
        Ok(json_value_to_nu_value(&json_value, span))
    }

    /// Reconstruct a Block from a nushell Value previously created by `to_nu_value`.
    pub fn from_nu_value(value: &Value) -> Result<Self, ShellError> {
        let span = value.span();
        let json_value = nu_value_to_json_value(value)?;
        serde_json::from_value(json_value).map_err(|e| ShellError::GenericError {
            error: "Failed to deserialize block".into(),
            msg: e.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
    }
}

/// Convert a serde_json::Value into a nushell Value.
fn json_value_to_nu_value(json: &serde_json::Value, span: Span) -> Value {
    match json {
        serde_json::Value::Null => Value::nothing(span),
        serde_json::Value::Bool(b) => Value::bool(*b, span),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::int(i, span)
            } else if let Some(u) = n.as_u64() {
                Value::int(u as i64, span)
            } else if let Some(f) = n.as_f64() {
                Value::float(f, span)
            } else {
                Value::nothing(span)
            }
        }
        serde_json::Value::String(s) => Value::string(s.clone(), span),
        serde_json::Value::Array(arr) => {
            let vals: Vec<Value> = arr
                .iter()
                .map(|v| json_value_to_nu_value(v, span))
                .collect();
            Value::list(vals, span)
        }
        serde_json::Value::Object(map) => {
            let mut record = Record::new();
            for (k, v) in map {
                record.push(k.clone(), json_value_to_nu_value(v, span));
            }
            Value::record(record, span)
        }
    }
}

/// Convert a nushell Value back into a serde_json::Value.
fn nu_value_to_json_value(value: &Value) -> Result<serde_json::Value, ShellError> {
    let span = value.span();
    match value {
        Value::Nothing { .. } => Ok(serde_json::Value::Null),
        Value::Bool { val, .. } => Ok(serde_json::Value::Bool(*val)),
        Value::Int { val, .. } => Ok(serde_json::Value::Number((*val).into())),
        Value::Float { val, .. } => {
            let n = serde_json::Number::from_f64(*val).ok_or_else(|| ShellError::GenericError {
                error: "Invalid float value".into(),
                msg: format!("cannot represent {val} as JSON number"),
                span: Some(span),
                help: None,
                inner: vec![],
            })?;
            Ok(serde_json::Value::Number(n))
        }
        Value::String { val, .. } => Ok(serde_json::Value::String(val.clone())),
        Value::List { vals, .. } => {
            let arr: Result<Vec<_>, _> = vals.iter().map(nu_value_to_json_value).collect();
            Ok(serde_json::Value::Array(arr?))
        }
        Value::Record { val, .. } => {
            let mut map = serde_json::Map::new();
            for (k, v) in val.iter() {
                map.insert(k.clone(), nu_value_to_json_value(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        _ => Err(ShellError::GenericError {
            error: "Unsupported value type for block deserialization".into(),
            msg: format!("cannot convert {} to JSON", value.get_type()),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }
}

impl<T> From<T> for Block
where
    T: Iterator<Item = Pipeline>,
{
    fn from(pipelines: T) -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            pipelines: pipelines.collect(),
            captures: vec![],
            redirect_env: false,
            ir_block: None,
            span: None,
        }
    }
}
