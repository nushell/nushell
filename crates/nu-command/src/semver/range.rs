use nu_protocol::{ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

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

    /// Requirements have no order, so only equal requirements compare, as `Equal`.
    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        let other = other
            .as_custom_value()
            .ok()?
            .as_any()
            .downcast_ref::<Self>()?;
        (self.requirement == other.requirement).then_some(Ordering::Equal)
    }

    fn hash_value(&self, mut state: &mut dyn Hasher) {
        self.requirement.hash(&mut state);
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

    #[test]
    fn equal_requirements_are_strict_eq_and_hash_equal() {
        use std::collections::hash_map::DefaultHasher;

        let range = |req| {
            Value::custom(
                Box::new(SemverRangeValue::new(
                    semver::VersionReq::parse(req).unwrap(),
                )),
                Span::test_data(),
            )
        };
        let hash = |val: &Value| {
            let mut hasher = DefaultHasher::new();
            val.hash(&mut hasher);
            hasher.finish()
        };

        let a = range("^1.2");
        let b = range("^1.2");
        assert!(a.strict_eq(&a));
        assert!(a.strict_eq(&b));
        assert_eq!(hash(&a), hash(&b));
        assert!(!a.strict_eq(&range("~1.2")));
    }
}
