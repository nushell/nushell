use super::Expression;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub expr: Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeBlock {
    pub attributes: Vec<Attribute>,
    pub item: Box<Expression>,
}
