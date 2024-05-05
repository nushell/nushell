use super::{Expression, RangeOperator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub from: Option<Expression>,
    pub next: Option<Expression>,
    pub to: Option<Expression>,
    pub operator: RangeOperator,
}
