use crate::Value;

pub struct Example {
    pub example: &'static str,
    pub description: &'static str,
    pub result: Option<Value>,
}
