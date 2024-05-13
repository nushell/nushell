use crate::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ValueWithUnit<Unit> {
    pub value: i64,
    pub value_span: Span,
    pub unit: Unit,
    pub unit_span: Span,
}
