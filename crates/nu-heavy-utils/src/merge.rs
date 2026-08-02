//! Merging behavior for [`Value`]s.
//!
//! This module contains the [`Merge`] trait which allows merging values into a single one using
//! different [merge strategies](MergeStrategy).
//! To ensure that the initial [`Value`]s are either both records or both tables, the [`typecheck`]
//! function may be used.
//!
//! This behavior is exposed via the nushell commands `merge` and `merge deep`.
//!
//! Execute a merge of two values and ensure both are records:
//! ```
//! use nu_heavy_utils::merge::{self, MergeStrategy, Merge};
//! # use nu_protocol::{test_record, Span, ShellError};
//! #
//! # fn main() -> Result<(), ShellError> {
//! let lhs = test_record! {
//!     "a" => 1,
//!     "b" => 2,
//! };
//! let rhs = test_record! {
//!     "a" => 42,
//!     "c" => 9,
//! };
//!
//! # let span = Span::test_data();
//! let strategy = MergeStrategy::Shallow;
//!
//! merge::typecheck(&lhs, &rhs, span)?;
//! let merged = lhs.merge(rhs, strategy, span)?;
//!
//! let expected = test_record! {
//!     "a" => 42,
//!     "b" => 2,
//!     "c" => 9,
//! };
//! assert_eq!(expected, merged);
//! # Ok(())
//! # }
//! ```

use nu_protocol::{Record, ShellError, Span, Type, Value};

type Table = Vec<Value>;

/// Controls how values are combined during a merge operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Key-value pairs present in `lhs` and `rhs` are overwritten by values in `rhs`.
    Shallow,

    /// Records are merged recursively, otherwise same behavior as shallow.
    Deep(ListMerge),
}

/// Defines how list values are handled when using [`MergeStrategy::Deep`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ListMerge {
    /// Lists in `lhs` are overwritten by lists in `rhs`.
    Overwrite,

    /// Lists of records are merged element-wise, other lists are overwritten by `rhs`.
    Elementwise,

    /// All lists are concatenated together, `lhs ++ rhs`.
    Append,

    /// All lists are concatenated together, `rhs ++ lhs`.
    Prepend,
}

mod sealed {
    pub trait Seal {}
}

/// Merge two values of related types into a single output.
///
/// Implementations treat `self` as the left-hand side (`lhs`) and `rhs` as the
/// right-hand side, with conflict resolution controlled by [`MergeStrategy`].
pub trait Merge: Sized + sealed::Seal {
    /// Merge `rhs` into `self` using the selected strategy.
    ///
    /// The provided `span` is used when constructing merged [`Value`] output or
    /// surfaced errors.
    fn merge(self, rhs: Self, strategy: MergeStrategy, span: Span) -> Result<Self, ShellError>;
}

impl sealed::Seal for Table {}
impl Merge for Table {
    /// Merge `rhs`-table into `self`-table, element-wise.
    ///
    /// ```
    /// # use nu_protocol::{test_table, Span};
    /// # use nu_heavy_utils::merge::{MergeStrategy, Merge};
    /// #
    /// let lhs = test_table![
    ///     ["a", "b"];
    ///     [ 12,  34],
    /// ];
    /// # let lhs = lhs.into_list().unwrap();
    /// let rhs = test_table![
    ///     ["a", "c"];
    ///     [ 56,  78],
    /// ];
    /// # let rhs = rhs.into_list().unwrap();
    ///
    /// # let span = Span::test_data();
    /// # let strategy = MergeStrategy::Shallow;
    /// let merged = lhs.merge(rhs, strategy, span).unwrap();
    ///
    /// let expected = test_table![
    ///     ["a", "b", "c"];
    ///     [ 56,  34,  78],
    /// ];
    /// # let expected = expected.into_list().unwrap();
    /// assert_eq!(merged, expected);
    /// ```
    fn merge(self, rhs: Self, strategy: MergeStrategy, span: Span) -> Result<Self, ShellError> {
        let lhs = self;
        let mut table_iter = rhs.into_iter();

        lhs.into_iter()
            .map(move |inp| match (inp.into_record(), table_iter.next()) {
                (Ok(rec), Some(to_merge)) => match to_merge.into_record() {
                    Ok(to_merge) => Ok(Value::record(rec.merge(to_merge, strategy, span)?, span)),
                    Err(error) => Ok(Value::error(error, span)),
                },
                (Ok(rec), None) => Ok(Value::record(rec, span)),
                (Err(error), _) => Ok(Value::error(error, span)),
            })
            .collect()
    }
}

impl sealed::Seal for Record {}
impl Merge for Record {
    /// Merge two records by key according to `strategy`.
    ///
    /// For shallow merges, colliding keys are replaced by values from `rhs`.
    /// For deep merges, nested values are recursively merged.
    fn merge(self, rhs: Self, strategy: MergeStrategy, span: Span) -> Result<Self, ShellError> {
        let mut lhs = self;
        match strategy {
            MergeStrategy::Shallow => {
                for (col, rval) in rhs.into_iter() {
                    lhs.insert(col, rval);
                }
            }
            strategy => {
                for (col, rval) in rhs.into_iter() {
                    // in order to both avoid cloning (possibly nested) record values and maintain the ordering of record keys, we can swap a temporary value into the source record.
                    // if we were to remove the value, the ordering would be messed up as we might not insert back into the original index
                    // it's okay to swap a temporary value in, since we know it will be replaced by the end of the function call
                    //
                    // use an error here instead of something like null so if this somehow makes it into the output, the bug will be immediately obvious
                    let failed_error = ShellError::NushellFailed {
                        msg: "Merge failed to properly replace internal temporary value".to_owned(),
                    };

                    let value = match lhs.insert(&col, Value::error(failed_error, span)) {
                        Some(lval) => lval.merge(rval, strategy, span)?,
                        None => rval,
                    };

                    lhs.insert(col, value);
                }
            }
        }
        Ok(lhs)
    }
}

impl sealed::Seal for Value {}
impl Merge for Value {
    /// Merge two [`Value`]s with record-aware and list-aware semantics.
    ///
    /// Errors are propagated, records are merged recursively when requested, and
    /// list behavior depends on the selected [`ListMerge`] strategy.
    fn merge(self, rhs: Self, strategy: MergeStrategy, span: Span) -> Result<Self, ShellError> {
        let lhs = self;
        match (strategy, lhs, rhs) {
            // Propagate errors
            (_, Value::Error { error, .. }, _) | (_, _, Value::Error { error, .. }) => Err(*error),
            // Merge records (shallow and deep)
            (
                MergeStrategy::Shallow | MergeStrategy::Deep(_),
                Value::Record { val: lhs, .. },
                Value::Record { val: rhs, .. },
            ) => Ok(Value::record(
                lhs.into_owned().merge(rhs.into_owned(), strategy, span)?,
                span,
            )),
            // Merge lists of records elementwise (tables and non-tables)
            // Match on shallow since this might be a top-level table
            (
                MergeStrategy::Shallow | MergeStrategy::Deep(ListMerge::Elementwise),
                lhs_list @ Value::List { .. },
                rhs_list @ Value::List { .. },
            ) if is_list_of_records(&lhs_list) && is_list_of_records(&rhs_list) => {
                let lhs = lhs_list
                    .into_list()
                    .expect("Value matched as list above, but is not a list");
                let rhs = rhs_list
                    .into_list()
                    .expect("Value matched as list above, but is not a list");
                Ok(Value::list(lhs.merge(rhs, strategy, span)?, span))
            }
            // Merge lists by appending
            (
                MergeStrategy::Deep(ListMerge::Append),
                Value::List { vals: lhs, .. },
                Value::List { vals: rhs, .. },
            ) => Ok(Value::list(lhs.into_iter().chain(rhs).collect(), span)),
            // Merge lists by prepending
            (
                MergeStrategy::Deep(ListMerge::Prepend),
                Value::List { vals: lhs, .. },
                Value::List { vals: rhs, .. },
            ) => Ok(Value::list(rhs.into_iter().chain(lhs).collect(), span)),
            // Use rhs value (shallow record merge, overwrite list merge, and general scalar merge)
            (_, _, val) => Ok(val),
        }
    }
}

/// Typecheck a merge operation.
///
/// Ensures that both arguments are records, tables, or lists of non-matching records.
pub fn typecheck(lhs: &Value, rhs: &Value, head: Span) -> Result<(), ShellError> {
    match (lhs.get_type(), rhs.get_type()) {
        (Type::Record { .. }, Type::Record { .. }) => Ok(()),
        (_, _) if is_list_of_records(lhs) && is_list_of_records(rhs) => Ok(()),
        other => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "input and argument to be both record or both table".to_string(),
            wrong_type: format!("{} and {}", other.0, other.1).to_string(),
            dst_span: head,
            src_span: lhs.span(),
        }),
    }
}

/// Test whether a value is a list of records.
///
/// This includes tables and non-tables.
fn is_list_of_records(val: &Value) -> bool {
    match val {
        list @ Value::List { .. } if matches!(list.get_type(), Type::Table { .. }) => true,
        // we want to include lists of records, but not lists of mixed types
        Value::List { vals, .. } => vals
            .iter()
            .map(Value::get_type)
            .all(|val| matches!(val, Type::Record { .. })),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{ListMerge, Merge, MergeStrategy, typecheck};
    use nu_protocol::{ShellError, Span, Value, test_record, test_table};

    #[test]
    fn shallow_record_merge_overwrites_and_preserves() -> Result<(), ShellError> {
        let lhs = test_record! {
            "a" => 1,
            "b" => 2,
        };
        let rhs = test_record! {
            "a" => 42,
            "c" => 9,
        };

        let merged = lhs.merge(rhs, MergeStrategy::Shallow, Span::test_data())?;

        assert_eq!(
            merged,
            test_record! {
                "a" => 42,
                "b" => 2,
                "c" => 9,
            }
        );
        Ok(())
    }

    #[test]
    fn deep_record_merge_recurses_nested_records() -> Result<(), ShellError> {
        let lhs = test_record! {
            "a" => test_record! {
                "b" => test_record! {
                    "c" => 1,
                    "d" => 2,
                },
            },
        };
        let rhs = test_record! {
            "a" => test_record! {
                "b" => test_record! {
                    "d" => 20,
                    "e" => 30,
                },
            },
        };

        let merged = lhs.merge(
            rhs,
            MergeStrategy::Deep(ListMerge::Overwrite),
            Span::test_data(),
        )?;

        assert_eq!(
            merged,
            test_record! {
                "a" => test_record! {
                    "b" => test_record! {
                        "c" => 1,
                        "d" => 20,
                        "e" => 30,
                    },
                },
            }
        );
        Ok(())
    }

    #[test]
    fn deep_list_merge_append() -> Result<(), ShellError> {
        let lhs = test_record! {
            "a" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
        };
        let rhs = test_record! {
            "a" => Value::test_list(vec![Value::test_int(3), Value::test_int(4)]),
        };

        let merged = lhs.merge(
            rhs,
            MergeStrategy::Deep(ListMerge::Append),
            Span::test_data(),
        )?;

        assert_eq!(
            merged,
            test_record! {
                "a" => Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ]),
            }
        );
        Ok(())
    }

    #[test]
    fn deep_list_merge_prepend() -> Result<(), ShellError> {
        let lhs = test_record! {
            "a" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
        };
        let rhs = test_record! {
            "a" => Value::test_list(vec![Value::test_int(3), Value::test_int(4)]),
        };

        let merged = lhs.merge(
            rhs,
            MergeStrategy::Deep(ListMerge::Prepend),
            Span::test_data(),
        )?;

        assert_eq!(
            merged,
            test_record! {
                "a" => Value::test_list(vec![
                    Value::test_int(3),
                    Value::test_int(4),
                    Value::test_int(1),
                    Value::test_int(2),
                ]),
            }
        );
        Ok(())
    }

    #[test]
    fn deep_list_merge_overwrite_for_scalar_lists() -> Result<(), ShellError> {
        let lhs = test_record! {
            "a" => Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_int(3),
            ]),
        };
        let rhs = test_record! {
            "a" => Value::test_list(vec![
                Value::test_int(4),
                Value::test_int(5),
                Value::test_int(6),
            ]),
        };

        let merged = lhs.merge(
            rhs,
            MergeStrategy::Deep(ListMerge::Overwrite),
            Span::test_data(),
        )?;

        assert_eq!(
            merged,
            test_record! {
                "a" => Value::test_list(vec![
                    Value::test_int(4),
                    Value::test_int(5),
                    Value::test_int(6),
                ]),
            }
        );
        Ok(())
    }

    #[test]
    fn table_merge_is_elementwise_and_preserves_unmatched_rows() -> Result<(), ShellError> {
        let lhs = test_table![
            ["a", "b"];
            [1, 2],
            [3, 4],
        ]
        .into_list()?;
        let rhs = test_table![
            ["a", "c"];
            [10, 20],
        ]
        .into_list()?;

        let merged = lhs.merge(rhs, MergeStrategy::Shallow, Span::test_data())?;

        assert_eq!(
            merged,
            vec![
                test_record! {
                    "a" => 10,
                    "b" => 2,
                    "c" => 20,
                },
                test_record! {
                    "a" => 3,
                    "b" => 4,
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn deep_elementwise_merges_nested_tables() -> Result<(), ShellError> {
        let lhs = test_record! {
            "inner" => test_table![
                ["a"];
                [test_record! {
                    "x" => 1,
                }],
                [test_record! {
                    "y" => 2,
                }],
            ],
        };
        let rhs = test_record! {
            "inner" => test_table![
                ["a"];
                [test_record! {
                    "z" => 3,
                }],
            ],
        };

        let merged = lhs.merge(
            rhs,
            MergeStrategy::Deep(ListMerge::Elementwise),
            Span::test_data(),
        )?;

        assert_eq!(
            merged,
            test_record! {
                "inner" => test_table![
                    ["a"];
                    [test_record! {
                        "x" => 1,
                        "z" => 3,
                    }],
                    [test_record! {
                        "y" => 2,
                    }],
                ],
            }
        );
        Ok(())
    }

    #[test]
    fn typecheck_rejects_record_and_scalar() {
        let result = typecheck(
            &test_record! {
                "a" => 1,
            },
            &Value::test_int(1),
            Span::test_data(),
        );

        assert!(matches!(
            result,
            Err(ShellError::OnlySupportsThisInputType { .. })
        ));
    }
}
