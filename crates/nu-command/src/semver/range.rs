use nu_protocol::{ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::any::Any;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemverRangeValue {
    pub requirement: semver::VersionReq,
}

#[typetag::serde]
impl nu_protocol::CustomValue for SemverRangeValue {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "semver-range".to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(self.requirement.to_string(), span))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl SemverRangeValue {
    pub fn new(requirement: semver::VersionReq) -> Self {
        Self { requirement }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::CustomValue;

    #[test]
    fn test_new() {
        let req = semver::VersionReq::parse(">=1.0.0").unwrap();
        let range = SemverRangeValue::new(req.clone());
        assert_eq!(range.requirement.to_string(), ">=1.0.0");
    }

    #[test]
    fn test_custom_value_trait() {
        let req = semver::VersionReq::parse("^1.2.3").unwrap();
        let range = SemverRangeValue::new(req);

        // Test type_name
        assert_eq!(range.type_name(), "semver-range");

        // Test to_base_value
        let base = range.to_base_value(Span::test_data()).unwrap();
        assert!(matches!(base, Value::String { val, .. } if val == "^1.2.3"));

        // Test clone_value
        let cloned = range.clone_value(Span::test_data());
        assert!(matches!(cloned, Value::Custom { .. }));

        // Test as_any
        let any = range.as_any();
        assert!(any.downcast_ref::<SemverRangeValue>().is_some());
    }

    #[test]
    fn test_various_requirements() {
        let test_cases = vec![
            ">=1.0.0",
            "<2.0.0",
            ">=1.0.0, <2.0.0",
            "^1.2.3",
            "~1.2",
            "1.2.3",
            "*",
        ];

        for req_str in test_cases {
            let req = semver::VersionReq::parse(req_str).unwrap();
            let range = SemverRangeValue::new(req);
            assert_eq!(range.type_name(), "semver-range");
        }
    }
}
