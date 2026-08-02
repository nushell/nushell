use nu_protocol::{
    CustomValue, ShellError, Span, Value,
    ast::{Comparison, Operator},
    casing::Casing,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cmp::Ordering;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
        match other {
            Value::Custom { val, .. } => {
                // Prefer a direct downcast when both values share the same crate instance.
                // Fall back to type_name + base-value parse so comparison still works when
                // TypeIds diverge (e.g. unit tests that pull nu-command in twice via
                // nu-test-support).
                if let Some(other) = val.as_any().downcast_ref::<SemverValue>() {
                    return self.version.partial_cmp(&other.version);
                }
                if val.type_name() == self.type_name() {
                    return val
                        .to_base_value(other.span())
                        .ok()
                        .and_then(|value| match value {
                            Value::String { val, .. } => semver::Version::parse(&val).ok(),
                            _ => None,
                        })
                        .and_then(|other_version| self.version.partial_cmp(&other_version));
                }
                None
            }
            Value::String { val, .. } => semver::Version::parse(val)
                .ok()
                .and_then(|other_version| self.version.partial_cmp(&other_version)),
            _ => None,
        }
    }

    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
        _optional: bool,
        casing: Casing,
    ) -> Result<Value, ShellError> {
        let col = match casing {
            Casing::Sensitive => column_name,
            Casing::Insensitive => column_name.to_lowercase(),
        };

        match col.as_str() {
            "major" => Ok(Value::int(self.version.major as i64, path_span)),
            "minor" => Ok(Value::int(self.version.minor as i64, path_span)),
            "patch" => Ok(Value::int(self.version.patch as i64, path_span)),
            "pre" => Ok(Value::string(self.version.pre.to_string(), path_span)),
            "build" => Ok(Value::string(self.version.build.to_string(), path_span)),
            _ => Err(ShellError::CantFindColumn {
                col_name: col,
                span: Some(path_span),
                src_span: self_span,
            }),
        }
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
            Operator::Comparison(
                comparison @ (Comparison::Equal
                | Comparison::NotEqual
                | Comparison::LessThan
                | Comparison::LessThanOrEqual
                | Comparison::GreaterThan
                | Comparison::GreaterThanOrEqual),
            ) => match CustomValue::partial_cmp(self, right) {
                Some(ordering) => {
                    let result = match comparison {
                        Comparison::Equal => ordering == Ordering::Equal,
                        Comparison::NotEqual => ordering != Ordering::Equal,
                        Comparison::LessThan => ordering == Ordering::Less,
                        Comparison::LessThanOrEqual => {
                            matches!(ordering, Ordering::Less | Ordering::Equal)
                        }
                        Comparison::GreaterThan => ordering == Ordering::Greater,
                        Comparison::GreaterThanOrEqual => {
                            matches!(ordering, Ordering::Greater | Ordering::Equal)
                        }
                        _ => unreachable!("matched only equality/ordering comparisons above"),
                    };
                    Ok(Value::bool(result, op))
                }
                None => Err(ShellError::OperatorIncompatibleTypes {
                    op: operator,
                    lhs: nu_protocol::Type::Custom("semver".into()),
                    rhs: right.get_type(),
                    op_span: op,
                    lhs_span,
                    rhs_span: right.span(),
                    help: Some("expected another semver or a valid version string"),
                }),
            },
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
            format!("{}.1", tag)
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

    pub fn set_build_metadata(&self, metadata: &str) -> Result<Self, ShellError> {
        let build = semver::BuildMetadata::new(metadata).map_err(|e| {
            ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                "Invalid build metadata",
                e.to_string(),
                Span::unknown(),
            ))
        })?;

        Ok(Self {
            version: semver::Version {
                major: self.version.major,
                minor: self.version.minor,
                patch: self.version.patch,
                pre: self.version.pre.clone(),
                build,
            },
        })
    }

    /// For use by tests and examples only.
    pub fn test_value(s: &str) -> Value {
        Value::test_custom_value(Box::new(Self {
            version: s
                .parse::<semver::Version>()
                .unwrap_or_else(|_| semver::Version::new(0, 0, 0)),
        }))
    }
}

impl<'a> TryFrom<&'a Value> for SemverValue {
    type Error = ShellError;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        let span = value.span();

        match value {
            Value::String { val, .. } => {
                semver::Version::parse(val)
                    .map(SemverValue::new)
                    .map_err(|e| ShellError::IncorrectValue {
                        msg: format!("Value is not a valid semver version: {e}"),
                        val_span: span,
                        call_span: span,
                    })
            }
            Value::Custom { val, .. } => {
                if let Some(semver) = val.as_any().downcast_ref::<Self>() {
                    Ok(semver.clone())
                } else {
                    Err(ShellError::CantConvert {
                        to_type: "semver".into(),
                        from_type: val.type_name(),
                        span,
                        help: None,
                    })
                }
            }
            x => Err(ShellError::CantConvert {
                to_type: "semver".into(),
                from_type: x.get_type().to_string(),
                span,
                help: None,
            }),
        }
    }
}

impl Deref for SemverValue {
    type Target = semver::Version;

    fn deref(&self) -> &Self::Target {
        &self.version
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::CustomValue;

    #[test]
    fn semver_custom_values_compare_equal_when_versions_match() {
        let expected = Value::custom(
            Box::new(SemverValue::new(semver::Version::parse("1.2.3").unwrap())),
            Span::test_data(),
        );
        let got = Value::custom(
            Box::new(SemverValue::new(semver::Version::parse("1.2.3").unwrap())),
            Span::test_data(),
        );

        assert_eq!(expected.partial_cmp(&got), Some(Ordering::Equal));
        assert_eq!(expected, got);
    }

    #[test]
    fn semver_bump_example_result_compares_equal_through_tester() -> nu_test_support::Result {
        let mut tester = nu_test_support::test();
        let got: Value = tester.run("'1.2.3' | into semver | semver bump major")?;
        let expected = SemverValue::test_value("2.0.0");

        assert_eq!(got.partial_cmp(&expected), Some(Ordering::Equal));
        assert_eq!(got, expected);
        Ok(())
    }

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
        assert_eq!(bumped.version.to_string(), "1.2.3-alpha.1");
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

        assert_eq!(CustomValue::partial_cmp(&v1, &val2), Some(Ordering::Less));
        assert_eq!(
            CustomValue::partial_cmp(&v2, &val1),
            Some(Ordering::Greater)
        );
        assert_eq!(CustomValue::partial_cmp(&v1, &val3), Some(Ordering::Equal));

        // Test with semver string input
        let string_val = Value::string("1.0.0", Span::test_data());
        assert_eq!(
            CustomValue::partial_cmp(&v1, &string_val),
            Some(Ordering::Equal)
        );

        // Test with non-semver string input
        let invalid_string_val = Value::string("not-a-version", Span::test_data());
        assert_eq!(CustomValue::partial_cmp(&v1, &invalid_string_val), None);
    }

    #[test]
    fn test_value_equality_for_semver_custom_values() {
        let expected = SemverValue::test_value("2.0.0");
        let actual = Value::custom(
            Box::new(SemverValue::new(parse_version("2.0.0"))),
            Span::test_data(),
        );

        assert_eq!(expected, actual);
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

    fn assert_bool_op(left: &SemverValue, comparison: Comparison, right: &Value, expected: bool) {
        let result = left
            .operation(
                Span::test_data(),
                Operator::Comparison(comparison),
                Span::test_data(),
                right,
            )
            .unwrap();
        assert!(matches!(result, Value::Bool { val, .. } if val == expected));
    }

    #[test]
    fn test_operation_comparisons() {
        let v201 = SemverValue::new(parse_version("2.0.1"));
        let v199 = SemverValue::new(parse_version("1.9.9"));
        let v100 = SemverValue::new(parse_version("1.0.0"));
        let v100_again = SemverValue::new(parse_version("1.0.0"));
        let v200 = SemverValue::new(parse_version("2.0.0"));
        let v100_alpha = SemverValue::new(parse_version("1.0.0-alpha"));

        let val201 = Value::custom(Box::new(v201.clone()), Span::test_data());
        let val199 = Value::custom(Box::new(v199.clone()), Span::test_data());
        let val100 = Value::custom(Box::new(v100.clone()), Span::test_data());
        let val100_again = Value::custom(Box::new(v100_again.clone()), Span::test_data());
        let val200 = Value::custom(Box::new(v200.clone()), Span::test_data());
        let val100_alpha = Value::custom(Box::new(v100_alpha.clone()), Span::test_data());

        // Discussion #18705 case: 2.0.1 is not less than 1.9.9
        assert_bool_op(&v201, Comparison::LessThan, &val199, false);
        assert_bool_op(&v201, Comparison::GreaterThan, &val199, true);
        assert_bool_op(&v199, Comparison::LessThan, &val201, true);

        // Equality / inequality
        assert_bool_op(&v100, Comparison::Equal, &val100_again, true);
        assert_bool_op(&v100, Comparison::NotEqual, &val200, true);
        assert_bool_op(&v100, Comparison::Equal, &val200, false);

        // Inclusive bounds
        assert_bool_op(&v100, Comparison::LessThanOrEqual, &val100_again, true);
        assert_bool_op(&v100, Comparison::GreaterThanOrEqual, &val100_again, true);
        assert_bool_op(&v100, Comparison::LessThanOrEqual, &val200, true);
        assert_bool_op(&v200, Comparison::GreaterThanOrEqual, &val100, true);

        // Prerelease is less than the corresponding release (semver spec)
        assert_bool_op(&v100_alpha, Comparison::LessThan, &val100, true);
        assert_bool_op(&v100, Comparison::GreaterThan, &val100_alpha, true);

        // String RHS
        let string_eq = Value::string("1.2.3", Span::test_data());
        let v123 = SemverValue::new(parse_version("1.2.3"));
        assert_bool_op(&v123, Comparison::Equal, &string_eq, true);
        assert_bool_op(
            &v123,
            Comparison::LessThan,
            &Value::string("2.0.0", Span::test_data()),
            true,
        );
        assert_bool_op(
            &v201,
            Comparison::GreaterThan,
            &Value::string("1.9.9", Span::test_data()),
            true,
        );

        // Incompatible RHS
        let int_rhs = Value::int(42, Span::test_data());
        assert!(
            v100.operation(
                Span::test_data(),
                Operator::Comparison(Comparison::Equal),
                Span::test_data(),
                &int_rhs,
            )
            .is_err()
        );
        let invalid_string = Value::string("not-a-version", Span::test_data());
        assert!(
            v100.operation(
                Span::test_data(),
                Operator::Comparison(Comparison::LessThan),
                Span::test_data(),
                &invalid_string,
            )
            .is_err()
        );
    }

    #[test]
    fn test_value_comparison_methods() {
        // Exercise the Value::{lt,gt,eq,...} path that dispatches to operation for custom values.
        let left = SemverValue::test_value("2.0.1");
        let right = SemverValue::test_value("1.9.9");
        let span = Span::test_data();

        assert!(matches!(
            left.gt(span, &right, span).unwrap(),
            Value::Bool { val: true, .. }
        ));
        assert!(matches!(
            left.lt(span, &right, span).unwrap(),
            Value::Bool { val: false, .. }
        ));
        assert!(matches!(
            left.eq(span, &left, span).unwrap(),
            Value::Bool { val: true, .. }
        ));
        assert!(matches!(
            left.ne(span, &right, span).unwrap(),
            Value::Bool { val: true, .. }
        ));
        assert!(matches!(
            left.gte(span, &right, span).unwrap(),
            Value::Bool { val: true, .. }
        ));
        assert!(matches!(
            right.lte(span, &left, span).unwrap(),
            Value::Bool { val: true, .. }
        ));
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
