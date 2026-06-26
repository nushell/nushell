use nu_protocol::{
    ShellError, Span, Value,
    ast::{Comparison, Operator},
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemverValue {
    pub version: semver::Version,
}

#[typetag::serde]
impl nu_protocol::CustomValue for SemverValue {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "semver".to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(self.version.to_string(), span))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        if let Value::Custom { val, .. } = other
            && let Some(other_semver) = val.as_any().downcast_ref::<SemverValue>()
        {
            return self.version.partial_cmp(&other_semver.version);
        }
        None
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        match operator {
            Operator::Comparison(Comparison::In) => {
                if let Value::Custom { val, .. } = right
                    && let Some(range) = val
                        .as_any()
                        .downcast_ref::<super::range::SemverRangeValue>()
                {
                    return Ok(Value::bool(range.requirement.matches(&self.version), op));
                }
                Err(ShellError::OperatorIncompatibleTypes {
                    op: operator,
                    lhs: nu_protocol::Type::Custom("semver".into()),
                    rhs: right.get_type(),
                    op_span: op,
                    lhs_span,
                    rhs_span: right.span(),
                    help: Some("expected a semver-range on the right side"),
                })
            }
            _ => Err(ShellError::OperatorUnsupportedType {
                op: operator,
                unsupported: nu_protocol::Type::Custom(self.type_name().into()),
                op_span: op,
                unsupported_span: lhs_span,
                help: None,
            }),
        }
    }
}

impl SemverValue {
    pub fn new(version: semver::Version) -> Self {
        Self { version }
    }

    pub fn bump_major(&self) -> Self {
        Self {
            version: semver::Version {
                major: self.version.major + 1,
                minor: 0,
                patch: 0,
                pre: semver::Prerelease::EMPTY,
                build: semver::BuildMetadata::EMPTY,
            },
        }
    }

    pub fn bump_minor(&self) -> Self {
        Self {
            version: semver::Version {
                major: self.version.major,
                minor: self.version.minor + 1,
                patch: 0,
                pre: semver::Prerelease::EMPTY,
                build: semver::BuildMetadata::EMPTY,
            },
        }
    }

    pub fn bump_patch(&self) -> Self {
        Self {
            version: semver::Version {
                major: self.version.major,
                minor: self.version.minor,
                patch: self.version.patch + 1,
                pre: semver::Prerelease::EMPTY,
                build: semver::BuildMetadata::EMPTY,
            },
        }
    }

    pub fn bump_prerelease(&self, tag: &str) -> Result<Self, ShellError> {
        let current_pre = self.version.pre.as_str();

        let new_pre = if current_pre.is_empty() {
            format!("{}.0", tag)
        } else if current_pre.starts_with(tag) {
            if let Some(dot_pos) = current_pre.rfind('.') {
                let suffix = &current_pre[dot_pos + 1..];
                if let Ok(num) = suffix.parse::<u64>() {
                    format!("{}.{}", tag, num + 1)
                } else {
                    format!("{}.1", tag)
                }
            } else {
                format!("{}.1", tag)
            }
        } else {
            format!("{}.0", tag)
        };

        let pre = semver::Prerelease::new(&new_pre).map_err(|e| {
            ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                "Invalid prerelease",
                e.to_string(),
                Span::unknown(),
            ))
        })?;

        Ok(Self {
            version: semver::Version {
                major: self.version.major,
                minor: self.version.minor,
                patch: self.version.patch,
                pre,
                build: self.version.build.clone(),
            },
        })
    }

    pub fn bump_release(&self) -> Self {
        Self {
            version: semver::Version {
                major: self.version.major,
                minor: self.version.minor,
                patch: self.version.patch,
                pre: semver::Prerelease::EMPTY,
                build: semver::BuildMetadata::EMPTY,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::CustomValue;

    fn parse_version(s: &str) -> semver::Version {
        semver::Version::parse(s).unwrap()
    }

    #[test]
    fn test_new() {
        let version = parse_version("1.2.3");
        let semver_val = SemverValue::new(version.clone());
        assert_eq!(semver_val.version, version);
    }

    #[test]
    fn test_bump_major() {
        let semver_val = SemverValue::new(parse_version("1.2.3"));
        let bumped = semver_val.bump_major();
        assert_eq!(bumped.version.to_string(), "2.0.0");

        // Test with prerelease and build metadata
        let semver_val = SemverValue::new(parse_version("1.2.3-alpha.1+build.2"));
        let bumped = semver_val.bump_major();
        assert_eq!(bumped.version.to_string(), "2.0.0");
    }

    #[test]
    fn test_bump_minor() {
        let semver_val = SemverValue::new(parse_version("1.2.3"));
        let bumped = semver_val.bump_minor();
        assert_eq!(bumped.version.to_string(), "1.3.0");

        // Test with prerelease
        let semver_val = SemverValue::new(parse_version("1.2.3-beta"));
        let bumped = semver_val.bump_minor();
        assert_eq!(bumped.version.to_string(), "1.3.0");
    }

    #[test]
    fn test_bump_patch() {
        let semver_val = SemverValue::new(parse_version("1.2.3"));
        let bumped = semver_val.bump_patch();
        assert_eq!(bumped.version.to_string(), "1.2.4");

        // Test with build metadata
        let semver_val = SemverValue::new(parse_version("1.2.3+build"));
        let bumped = semver_val.bump_patch();
        assert_eq!(bumped.version.to_string(), "1.2.4");
    }

    #[test]
    fn test_bump_prerelease_empty() {
        let semver_val = SemverValue::new(parse_version("1.2.3"));
        let bumped = semver_val.bump_prerelease("alpha").unwrap();
        assert_eq!(bumped.version.to_string(), "1.2.3-alpha.0");
    }

    #[test]
    fn test_bump_prerelease_same_tag() {
        let semver_val = SemverValue::new(parse_version("1.2.3-alpha.0"));
        let bumped = semver_val.bump_prerelease("alpha").unwrap();
        assert_eq!(bumped.version.to_string(), "1.2.3-alpha.1");

        let semver_val = SemverValue::new(parse_version("1.2.3-alpha.5"));
        let bumped = semver_val.bump_prerelease("alpha").unwrap();
        assert_eq!(bumped.version.to_string(), "1.2.3-alpha.6");
    }

    #[test]
    fn test_bump_prerelease_different_tag() {
        let semver_val = SemverValue::new(parse_version("1.2.3-alpha.1"));
        let bumped = semver_val.bump_prerelease("beta").unwrap();
        assert_eq!(bumped.version.to_string(), "1.2.3-beta.0");
    }

    #[test]
    fn test_bump_prerelease_no_number() {
        let semver_val = SemverValue::new(parse_version("1.2.3-alpha"));
        let bumped = semver_val.bump_prerelease("alpha").unwrap();
        assert_eq!(bumped.version.to_string(), "1.2.3-alpha.1");
    }

    #[test]
    fn test_bump_release() {
        let semver_val = SemverValue::new(parse_version("1.2.3-alpha.1+build.2"));
        let bumped = semver_val.bump_release();
        assert_eq!(bumped.version.to_string(), "1.2.3");

        let semver_val = SemverValue::new(parse_version("1.2.3"));
        let bumped = semver_val.bump_release();
        assert_eq!(bumped.version.to_string(), "1.2.3");
    }

    #[test]
    fn test_partial_cmp() {
        let v1 = SemverValue::new(parse_version("1.0.0"));
        let v2 = SemverValue::new(parse_version("2.0.0"));
        let v3 = SemverValue::new(parse_version("1.0.0"));

        let val2 = Value::custom(Box::new(v2.clone()), Span::test_data());
        let val1 = Value::custom(Box::new(v1.clone()), Span::test_data());
        let val3 = Value::custom(Box::new(v3.clone()), Span::test_data());

        assert_eq!(v1.partial_cmp(&val2), Some(Ordering::Less));
        assert_eq!(v2.partial_cmp(&val1), Some(Ordering::Greater));
        assert_eq!(v1.partial_cmp(&val3), Some(Ordering::Equal));

        // Test with non-semver value
        let string_val = Value::string("1.0.0", Span::test_data());
        assert_eq!(v1.partial_cmp(&string_val), None);
    }

    #[test]
    fn test_operation_in() {
        use crate::semver::range::SemverRangeValue;

        let version = SemverValue::new(parse_version("1.2.3"));
        let range = SemverRangeValue::new(semver::VersionReq::parse(">=1.0.0").unwrap());

        let range_val = Value::custom(Box::new(range), Span::test_data());

        let result = version
            .operation(
                Span::test_data(),
                Operator::Comparison(Comparison::In),
                Span::test_data(),
                &range_val,
            )
            .unwrap();

        assert!(matches!(result, Value::Bool { val: true, .. }));

        // Test with non-matching range
        let range = SemverRangeValue::new(semver::VersionReq::parse(">=2.0.0").unwrap());
        let range_val = Value::custom(Box::new(range), Span::test_data());

        let result = version
            .operation(
                Span::test_data(),
                Operator::Comparison(Comparison::In),
                Span::test_data(),
                &range_val,
            )
            .unwrap();

        assert!(matches!(result, Value::Bool { val: false, .. }));
    }

    #[test]
    fn test_operation_unsupported() {
        let version = SemverValue::new(parse_version("1.2.3"));
        let other = Value::int(42, Span::test_data());

        let result = version.operation(
            Span::test_data(),
            Operator::Math(nu_protocol::ast::Math::Add),
            Span::test_data(),
            &other,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_custom_value_trait() {
        let version = SemverValue::new(parse_version("1.2.3"));

        // Test type_name
        assert_eq!(version.type_name(), "semver");

        // Test to_base_value
        let base = version.to_base_value(Span::test_data()).unwrap();
        assert!(matches!(base, Value::String { val, .. } if val == "1.2.3"));

        // Test clone_value
        let cloned = version.clone_value(Span::test_data());
        assert!(matches!(cloned, Value::Custom { .. }));

        // Test as_any
        let any = version.as_any();
        assert!(any.downcast_ref::<SemverValue>().is_some());
    }
}
