use super::Expression;
use crate::FutureSpanId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyword {
    pub keyword: Box<[u8]>,
    pub span: FutureSpanId,
    pub expr: Expression,
}
