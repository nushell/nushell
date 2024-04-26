use super::Expression;
use crate::{Spanned, Unit};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueWithUnit {
    pub expr: Expression,
    pub unit: Spanned<Unit>,
}
