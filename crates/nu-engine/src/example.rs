use nu_protocol::Value;

pub struct Example {
    pub example: &'static str,
    pub description: &'static str,
    pub result: Option<Vec<Value>>,
}
