pub(crate) mod shape;

use crate::context::CommandRegistry;
use crate::evaluate::evaluate_baseline_expr;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use derive_new::new;
use log::trace;
use nu_errors::ShellError;
use nu_parser::{hir, CompareOperator};
use nu_protocol::{
    Evaluate, EvaluateTrait, Primitive, Scope, ShellTypeName, SpannedTypeName, TaggedDictBuilder,
    UntaggedValue, Value,
};
use nu_source::{Tag, Text};
use nu_value_ext::ValueExt;
use num_bigint::BigInt;
use num_traits::Zero;
use query_interface::{interfaces, vtable_for, ObjectHash};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, new, Serialize)]
pub struct Operation {
    pub(crate) left: Value,
    pub(crate) operator: CompareOperator,
    pub(crate) right: Value,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Serialize, Deserialize, new)]
pub struct Block {
    pub(crate) expressions: Vec<hir::Expression>,
    pub(crate) source: Text,
    pub(crate) tag: Tag,
}

interfaces!(Block: dyn ObjectHash);

#[typetag::serde]
impl EvaluateTrait for Block {
    fn invoke(&self, scope: &Scope) -> Result<Value, ShellError> {
        if self.expressions.is_empty() {
            return Ok(UntaggedValue::nothing().into_value(&self.tag));
        }

        let mut last = Ok(UntaggedValue::nothing().into_value(&self.tag));

        trace!(
            "EXPRS = {:?}",
            self.expressions
                .iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
        );

        for expr in self.expressions.iter() {
            last = evaluate_baseline_expr(&expr, &CommandRegistry::empty(), &scope, &self.source)
        }

        last
    }

    fn clone_box(&self) -> Evaluate {
        let block = self.clone();
        Evaluate::new(block)
    }
}

#[derive(Serialize, Deserialize)]
pub enum Switch {
    Present,
    Absent,
}

impl std::convert::TryFrom<Option<&Value>> for Switch {
    type Error = ShellError;

    fn try_from(value: Option<&Value>) -> Result<Switch, ShellError> {
        match value {
            None => Ok(Switch::Absent),
            Some(value) => match &value.value {
                UntaggedValue::Primitive(Primitive::Boolean(true)) => Ok(Switch::Present),
                _ => Err(ShellError::type_error("Boolean", value.spanned_type_name())),
            },
        }
    }
}

#[allow(unused)]
pub(crate) fn select_fields(obj: &Value, fields: &[String], tag: impl Into<Tag>) -> Value {
    let mut out = TaggedDictBuilder::new(tag);

    let descs = obj.data_descriptors();

    for column_name in fields {
        match descs.iter().find(|d| *d == column_name) {
            None => out.insert_untagged(column_name, UntaggedValue::nothing()),
            Some(desc) => out.insert_value(desc.clone(), obj.get_data(desc).borrow().clone()),
        }
    }

    out.into_value()
}

pub(crate) fn reject_fields(obj: &Value, fields: &[String], tag: impl Into<Tag>) -> Value {
    let mut out = TaggedDictBuilder::new(tag);

    let descs = obj.data_descriptors();

    for desc in descs {
        if fields.iter().any(|field| *field == desc) {
            continue;
        } else {
            out.insert_value(desc.clone(), obj.get_data(&desc).borrow().clone())
        }
    }

    out.into_value()
}

pub(crate) enum CompareValues {
    Ints(BigInt, BigInt),
    Decimals(BigDecimal, BigDecimal),
    String(String, String),
    Date(DateTime<Utc>, DateTime<Utc>),
    DateDuration(DateTime<Utc>, u64),
}

impl CompareValues {
    pub fn compare(&self) -> std::cmp::Ordering {
        match self {
            CompareValues::Ints(left, right) => left.cmp(right),
            CompareValues::Decimals(left, right) => left.cmp(right),
            CompareValues::String(left, right) => left.cmp(right),
            CompareValues::Date(left, right) => left.cmp(right),
            CompareValues::DateDuration(left, right) => {
                use std::time::Duration;

                // Create the datetime we're comparing against, as duration is an offset from now
                let right: DateTime<Utc> = (SystemTime::now() - Duration::from_secs(*right)).into();
                right.cmp(left)
            }
        }
    }
}

pub(crate) fn coerce_compare(
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<CompareValues, (&'static str, &'static str)> {
    match (left, right) {
        (UntaggedValue::Primitive(left), UntaggedValue::Primitive(right)) => {
            coerce_compare_primitive(left, right)
        }

        _ => Err((left.type_name(), right.type_name())),
    }
}

fn coerce_compare_primitive(
    left: &Primitive,
    right: &Primitive,
) -> Result<CompareValues, (&'static str, &'static str)> {
    use Primitive::*;

    Ok(match (left, right) {
        (Int(left), Int(right)) => CompareValues::Ints(left.clone(), right.clone()),
        (Int(left), Decimal(right)) => {
            CompareValues::Decimals(BigDecimal::zero() + left, right.clone())
        }
        (Int(left), Bytes(right)) => CompareValues::Ints(left.clone(), BigInt::from(*right)),
        (Decimal(left), Decimal(right)) => CompareValues::Decimals(left.clone(), right.clone()),
        (Decimal(left), Int(right)) => {
            CompareValues::Decimals(left.clone(), BigDecimal::zero() + right)
        }
        (Decimal(left), Bytes(right)) => {
            CompareValues::Decimals(left.clone(), BigDecimal::from(*right))
        }
        (Bytes(left), Int(right)) => CompareValues::Ints(BigInt::from(*left), right.clone()),
        (Bytes(left), Decimal(right)) => {
            CompareValues::Decimals(BigDecimal::from(*left), right.clone())
        }
        (String(left), String(right)) => CompareValues::String(left.clone(), right.clone()),
        (Line(left), String(right)) => CompareValues::String(left.clone(), right.clone()),
        (String(left), Line(right)) => CompareValues::String(left.clone(), right.clone()),
        (Line(left), Line(right)) => CompareValues::String(left.clone(), right.clone()),
        (Date(left), Date(right)) => CompareValues::Date(*left, *right),
        (Date(left), Duration(right)) => CompareValues::DateDuration(*left, *right),
        _ => return Err((left.type_name(), right.type_name())),
    })
}
#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use nu_errors::ShellError;
    use nu_protocol::{ColumnPath as ColumnPathValue, PathMember, UntaggedValue, Value};
    use nu_source::*;
    use nu_value_ext::{as_column_path, ValueExt};
    use num_bigint::BigInt;

    fn string(input: impl Into<String>) -> Value {
        UntaggedValue::string(input.into()).into_untagged_value()
    }

    fn int(input: impl Into<BigInt>) -> Value {
        UntaggedValue::int(input.into()).into_untagged_value()
    }

    fn row(entries: IndexMap<String, Value>) -> Value {
        UntaggedValue::row(entries).into_untagged_value()
    }

    fn table(list: &[Value]) -> Value {
        UntaggedValue::table(list).into_untagged_value()
    }

    fn error_callback(
        reason: &'static str,
    ) -> impl FnOnce((&Value, &PathMember, ShellError)) -> ShellError {
        move |(_obj_source, _column_path_tried, _err)| ShellError::unimplemented(reason)
    }

    fn column_path(paths: &[Value]) -> Result<Tagged<ColumnPathValue>, ShellError> {
        as_column_path(&table(paths))
    }

    #[test]
    fn gets_matching_field_from_a_row() -> Result<(), ShellError> {
        let row = UntaggedValue::row(indexmap! {
            "amigos".into() => table(&[string("andres"),string("jonathan"),string("yehuda")])
        })
        .into_untagged_value();

        assert_eq!(
            row.get_data_by_key("amigos".spanned_unknown())
                .ok_or_else(|| ShellError::unexpected("Failure during testing"))?,
            table(&[string("andres"), string("jonathan"), string("yehuda")])
        );

        Ok(())
    }

    #[test]
    fn gets_matching_field_from_nested_rows_inside_a_row() -> Result<(), ShellError> {
        let field_path = column_path(&[string("package"), string("version")]);

        let (version, tag) = string("0.4.0").into_parts();

        let value = UntaggedValue::row(indexmap! {
            "package".into() =>
                row(indexmap! {
                    "name".into()    =>     string("nu"),
                    "version".into() =>  string("0.4.0")
                })
        });

        assert_eq!(
            *value.into_value(tag).get_data_by_column_path(
                &field_path?.item,
                Box::new(error_callback("package.version"))
            )?,
            version
        );

        Ok(())
    }

    #[test]
    fn gets_first_matching_field_from_rows_with_same_field_inside_a_table() -> Result<(), ShellError>
    {
        let field_path = column_path(&[string("package"), string("authors"), string("name")]);

        let (_, tag) = string("Andrés N. Robalino").into_parts();

        let value = UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "name".into() => string("nu"),
                "version".into() => string("0.4.0"),
                "authors".into() => table(&[
                    row(indexmap!{"name".into() => string("Andrés N. Robalino")}),
                    row(indexmap!{"name".into() => string("Jonathan Turner")}),
                    row(indexmap!{"name".into() => string("Yehuda Katz")})
                ])
            })
        });

        assert_eq!(
            value.into_value(tag).get_data_by_column_path(
                &field_path?.item,
                Box::new(error_callback("package.authors.name"))
            )?,
            table(&[
                string("Andrés N. Robalino"),
                string("Jonathan Turner"),
                string("Yehuda Katz")
            ])
        );

        Ok(())
    }

    #[test]
    fn column_path_that_contains_just_a_number_gets_a_row_from_a_table() -> Result<(), ShellError> {
        let field_path = column_path(&[string("package"), string("authors"), int(0)]);

        let (_, tag) = string("Andrés N. Robalino").into_parts();

        let value = UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "name".into() => string("nu"),
                "version".into() => string("0.4.0"),
                "authors".into() => table(&[
                    row(indexmap!{"name".into() => string("Andrés N. Robalino")}),
                    row(indexmap!{"name".into() => string("Jonathan Turner")}),
                    row(indexmap!{"name".into() => string("Yehuda Katz")})
                ])
            })
        });

        assert_eq!(
            *value.into_value(tag).get_data_by_column_path(
                &field_path?.item,
                Box::new(error_callback("package.authors.0"))
            )?,
            UntaggedValue::row(indexmap! {
                "name".into() => string("Andrés N. Robalino")
            })
        );

        Ok(())
    }

    #[test]
    fn column_path_that_contains_just_a_number_gets_a_row_from_a_row() -> Result<(), ShellError> {
        let field_path = column_path(&[string("package"), string("authors"), string("0")]);

        let (_, tag) = string("Andrés N. Robalino").into_parts();

        let value = UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "name".into() => string("nu"),
                "version".into() => string("0.4.0"),
                "authors".into() => row(indexmap! {
                    "0".into() => row(indexmap!{"name".into() => string("Andrés N. Robalino")}),
                    "1".into() => row(indexmap!{"name".into() => string("Jonathan Turner")}),
                    "2".into() => row(indexmap!{"name".into() => string("Yehuda Katz")}),
                })
            })
        });

        assert_eq!(
            *value.into_value(tag).get_data_by_column_path(
                &field_path?.item,
                Box::new(error_callback("package.authors.\"0\""))
            )?,
            UntaggedValue::row(indexmap! {
                "name".into() => string("Andrés N. Robalino")
            })
        );

        Ok(())
    }

    #[test]
    fn replaces_matching_field_from_a_row() -> Result<(), ShellError> {
        let field_path = column_path(&[string("amigos")]);

        let sample = UntaggedValue::row(indexmap! {
            "amigos".into() => table(&[
                string("andres"),
                string("jonathan"),
                string("yehuda"),
            ]),
        });

        let replacement = string("jonas");

        let actual = sample
            .into_untagged_value()
            .replace_data_at_column_path(&field_path?.item, replacement)
            .ok_or_else(|| ShellError::untagged_runtime_error("Could not replace column"))?;

        assert_eq!(actual, row(indexmap! {"amigos".into() => string("jonas")}));

        Ok(())
    }

    #[test]
    fn replaces_matching_field_from_nested_rows_inside_a_row() -> Result<(), ShellError> {
        let field_path = column_path(&[
            string("package"),
            string("authors"),
            string("los.3.caballeros"),
        ]);

        let sample = UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "authors".into() => row(indexmap! {
                    "los.3.mosqueteros".into() => table(&[string("andres::yehuda::jonathan")]),
                    "los.3.amigos".into() => table(&[string("andres::yehuda::jonathan")]),
                    "los.3.caballeros".into() => table(&[string("andres::yehuda::jonathan")])
                })
            })
        });

        let replacement = table(&[string("yehuda::jonathan::andres")]);
        let tag = replacement.tag.clone();

        let actual = sample
            .into_value(&tag)
            .replace_data_at_column_path(&field_path?.item, replacement.clone())
            .ok_or_else(|| {
                ShellError::labeled_error(
                    "Could not replace column",
                    "could not replace column",
                    &tag,
                )
            })?;

        assert_eq!(
            actual,
            UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "authors".into() => row(indexmap! {
                    "los.3.mosqueteros".into() => table(&[string("andres::yehuda::jonathan")]),
                    "los.3.amigos".into()      => table(&[string("andres::yehuda::jonathan")]),
                    "los.3.caballeros".into()  => replacement})})})
            .into_value(tag)
        );

        Ok(())
    }
    #[test]
    fn replaces_matching_field_from_rows_inside_a_table() -> Result<(), ShellError> {
        let field_path = column_path(&[
            string("shell_policy"),
            string("releases"),
            string("nu.version.arepa"),
        ]);

        let sample = UntaggedValue::row(indexmap! {
            "shell_policy".into() => row(indexmap! {
                "releases".into() => table(&[
                    row(indexmap! {
                        "nu.version.arepa".into() => row(indexmap! {
                            "code".into() => string("0.4.0"), "tag_line".into() => string("GitHub-era")
                        })
                    }),
                    row(indexmap! {
                        "nu.version.taco".into() => row(indexmap! {
                            "code".into() => string("0.3.0"), "tag_line".into() => string("GitHub-era")
                        })
                    }),
                    row(indexmap! {
                        "nu.version.stable".into() => row(indexmap! {
                            "code".into() => string("0.2.0"), "tag_line".into() => string("GitHub-era")
                        })
                    })
                ])
            })
        });

        let replacement = row(indexmap! {
            "code".into() => string("0.5.0"),
            "tag_line".into() => string("CABALLEROS")
        });
        let tag = replacement.tag.clone();

        let actual = sample
            .into_value(tag.clone())
            .replace_data_at_column_path(&field_path?.item, replacement.clone())
            .ok_or_else(|| {
                ShellError::labeled_error(
                    "Could not replace column",
                    "could not replace column",
                    &tag,
                )
            })?;

        assert_eq!(
            actual,
            UntaggedValue::row(indexmap! {
                "shell_policy".into() => row(indexmap! {
                    "releases".into() => table(&[
                        row(indexmap! {
                            "nu.version.arepa".into() => replacement
                        }),
                        row(indexmap! {
                            "nu.version.taco".into() => row(indexmap! {
                                "code".into() => string("0.3.0"), "tag_line".into() => string("GitHub-era")
                            })
                        }),
                        row(indexmap! {
                            "nu.version.stable".into() => row(indexmap! {
                                "code".into() => string("0.2.0"), "tag_line".into() => string("GitHub-era")
                            })
                        })
                    ])
                })
            }).into_value(&tag)
        );

        Ok(())
    }
}
