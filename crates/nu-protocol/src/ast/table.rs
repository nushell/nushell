use super::Expression;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub columns: Box<[Expression]>,
    pub rows: Box<[Box<[Expression]>]>,
}
