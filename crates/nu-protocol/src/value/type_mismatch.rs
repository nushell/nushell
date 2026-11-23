//! Field-level explanations for a value that doesn't match a type annotation.

use crate::{CollectionColumns, CompareTypes, Record, Type, Value};
use nu_utils::{escape_quote_string, needs_quoting};

impl Value {
    /// Explains which record fields and list items keep this value from matching `ty`.
    ///
    /// Each problem is one line, for example "missing field `h: int`" or
    /// "field `config.chapter`: expected int, got string". A record reports every field that
    /// fails, and a list or table reports only its first failing item. Fields that match are
    /// never named, so an empty list in a `list<string>` field doesn't show up.
    ///
    /// Returns `None` when the value matches, and when the value isn't a record or list, because
    /// then the error's own "expected X, got Y" already says everything.
    ///
    /// Meant for error messages: call it only after a type check has failed.
    pub fn explain_type_mismatch(&self, ty: &Type) -> Option<String> {
        if self.is_subtype_of(ty) {
            return None;
        }
        let mut lines = Vec::new();
        explain_children(self, ty, "", &mut lines);
        (!lines.is_empty()).then(|| lines.join("\n"))
    }
}

/// Pushes the lines for the fields or items of `value`, which sits at `path`, that don't match
/// `ty`. Pushes nothing unless `value` is a record or list and `ty` is a type of the same kind.
fn explain_children(value: &Value, ty: &Type, path: &str, lines: &mut Vec<String>) {
    match (value, ty) {
        (Value::Record { val, .. }, Type::Record(cols)) => explain_record(val, cols, path, lines),
        (Value::List { vals, .. }, Type::List(item_ty)) => explain_list(vals, item_ty, path, lines),
        (Value::List { vals, .. }, Type::Table(cols)) => explain_table(vals, cols, path, lines),
        _ => {}
    }
}

/// Pushes a line for each field of `cols` that `record` lacks or doesn't match.
fn explain_record(
    record: &Record,
    cols: &CollectionColumns<Type>,
    path: &str,
    lines: &mut Vec<String>,
) {
    for (name, field_ty) in cols {
        let field_path = join_field(path, name);
        match record.get(name) {
            None => lines.push(format!("missing field `{field_path}: {field_ty}`")),
            Some(field) if !field.is_subtype_of(field_ty) => {
                explain_value(field, field_ty, &field_path, "field", lines)
            }
            Some(_) => {}
        }
    }
}

/// Pushes the lines for the first item of `vals` that doesn't match `item_ty`.
fn explain_list(vals: &[Value], item_ty: &Type, path: &str, lines: &mut Vec<String>) {
    if let Some((index, item)) = vals
        .iter()
        .enumerate()
        .find(|(_, item)| !item.is_subtype_of(item_ty))
    {
        explain_value(item, item_ty, &join_index(path, index), "item", lines);
    }
}

/// Pushes the lines for the first item of `vals` that isn't a record matching `cols`.
fn explain_table(
    vals: &[Value],
    cols: &CollectionColumns<Type>,
    path: &str,
    lines: &mut Vec<String>,
) {
    let mismatch = vals.iter().enumerate().find(|(_, item)| match item {
        Value::Record { val, .. } => !val.is_subtype_of(cols),
        _ => true,
    });
    if let Some((index, item)) = mismatch {
        explain_value(
            item,
            &Type::Record(cols.clone()),
            &join_index(path, index),
            "item",
            lines,
        );
    }
}

/// Pushes the lines for `value`, which sits at `path` and doesn't match `ty`.
///
/// Records and lists are explained by their fields and items. Anything else, or a walk that
/// found nothing to name, gets one line such as "field `a.b`: expected int, got string", where
/// `kind` is "field" or "item".
fn explain_value(value: &Value, ty: &Type, path: &str, kind: &str, lines: &mut Vec<String>) {
    let before = lines.len();
    explain_children(value, ty, path, lines);
    if lines.len() == before {
        lines.push(format!(
            "{kind} `{path}`: expected {ty}, got {}",
            value.get_type()
        ));
    }
}

/// Appends a record field to a dotted path, quoting the name wherever a cell path would quote it,
/// so a name like `1` or `a.b` isn't read as a list index or a nested field.
///
/// [`escape_quote_string`] leaves a backtick or a newline in a name as is.
fn join_field(path: &str, name: &str) -> String {
    let name = if needs_quoting(name) {
        escape_quote_string(name)
    } else {
        name.to_owned()
    };
    if path.is_empty() {
        name
    } else {
        format!("{path}.{name}")
    }
}

/// Appends a list index to a dotted path.
fn join_index(path: &str, index: usize) -> String {
    if path.is_empty() {
        index.to_string()
    } else {
        format!("{path}.{index}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{record, test_record};
    use rstest::rstest;

    fn record_ty<const N: usize>(cols: [(&str, Type); N]) -> Type {
        Type::Record(cols.into())
    }

    fn explain(value: Value, ty: &Type) -> Option<String> {
        let explanation = value.explain_type_mismatch(ty);
        // Only a value that fails the check gets an explanation.
        if explanation.is_some() {
            assert!(!value.is_subtype_of(ty));
        }
        explanation
    }

    #[test]
    fn names_only_the_missing_field() {
        // From #19501: the empty lists match `list<int>` and `list<string>` and stay unnamed.
        let ty = record_ty([
            ("a", Type::Int),
            ("c", Type::list(Type::Int)),
            ("d", Type::list(Type::String)),
            ("h", Type::Int),
        ]);
        let value = test_record! {
            "a" => Value::test_int(1),
            "c" => Value::test_list(vec![]),
            "d" => Value::test_list(vec![]),
        };
        assert_eq!(
            explain(value, &ty).as_deref(),
            Some("missing field `h: int`")
        );
    }

    #[test]
    fn names_every_failing_field() {
        let ty = record_ty([("a", Type::Int), ("b", Type::String), ("c", Type::Int)]);
        let value = test_record! {
            "a" => Value::test_string("x"),
            "b" => Value::test_string("y"),
        };
        assert_eq!(
            explain(value, &ty).as_deref(),
            Some("field `a`: expected int, got string\nmissing field `c: int`")
        );
    }

    #[test]
    fn ignores_extra_fields() {
        let ty = record_ty([("a", Type::Int)]);
        let value = test_record! {
            "a" => Value::test_bool(true),
            "extra" => Value::test_int(1),
        };
        assert_eq!(
            explain(value, &ty).as_deref(),
            Some("field `a`: expected int, got bool")
        );
    }

    #[test]
    fn follows_nested_records() {
        let ty = record_ty([(
            "config",
            record_ty([("chapter", Type::Int), ("title", Type::String)]),
        )]);
        let value = test_record! {
            "config" => Value::test_record(record! {
                "chapter" => Value::test_string("one"),
            }),
        };
        assert_eq!(
            explain(value, &ty).as_deref(),
            Some(
                "field `config.chapter`: expected int, got string\n\
                 missing field `config.title: string`"
            )
        );
    }

    #[test]
    fn names_the_first_failing_list_item() {
        let ty = record_ty([("c", Type::list(Type::Int))]);
        let value = test_record! {
            "c" => Value::test_list(vec![
                Value::test_int(1),
                Value::test_string("x"),
                Value::test_string("y"),
            ]),
        };
        assert_eq!(
            explain(value, &ty).as_deref(),
            Some("item `c.1`: expected int, got string")
        );
    }

    #[test]
    fn follows_table_rows() {
        let ty = Type::Table([("name", Type::String), ("size", Type::Int)].into());
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "name" => Value::test_string("a"),
                "size" => Value::test_int(1),
            }),
            Value::test_record(record! {
                "name" => Value::test_string("b"),
            }),
            Value::test_int(3),
        ]);
        assert_eq!(
            explain(value, &ty).as_deref(),
            Some("missing field `1.size: int`")
        );
    }

    #[test]
    fn non_record_table_row() {
        let ty = Type::Table([("name", Type::String)].into());
        let value = Value::test_list(vec![Value::test_int(3)]);
        assert_eq!(
            explain(value, &ty).as_deref(),
            Some("item `0`: expected record<name: string>, got int")
        );
    }

    #[test]
    fn quotes_ambiguous_field_names() {
        // Unquoted, `1` would read as a list index and `b?` as an optional cell path member.
        let ty = record_ty([
            ("a.b", Type::Int),
            ("first name", Type::String),
            ("1", Type::Int),
            ("b?", Type::Int),
        ]);
        let value = test_record! {
            "a.b" => Value::test_string("x"),
        };
        assert_eq!(
            explain(value, &ty).as_deref(),
            Some(
                "field `\"a.b\"`: expected int, got string\n\
                 missing field `\"first name\": string`\n\
                 missing field `\"1\": int`\n\
                 missing field `\"b?\": int`"
            )
        );
    }

    #[test]
    fn stops_at_oneof() {
        let ty = record_ty([(
            "a",
            Type::one_of([record_ty([("x", Type::Int)]), Type::Nothing]),
        )]);
        let value = test_record! {
            "a" => Value::test_record(record! { "x" => Value::test_string("s") }),
        };
        assert_eq!(
            explain(value, &ty).as_deref(),
            Some("field `a`: expected oneof<record<x: int>, nothing>, got record<x: string>")
        );
    }

    #[rstest]
    #[case::matching_record(
        test_record! { "a" => Value::test_list(vec![]) },
        record_ty([("a", Type::list(Type::String))]),
    )]
    #[case::scalar_mismatch(Value::test_int(1), Type::String)]
    #[case::kind_mismatch(Value::test_int(1), record_ty([("a", Type::Int)]))]
    fn nothing_to_explain(#[case] value: Value, #[case] ty: Type) {
        assert_eq!(explain(value, &ty), None);
    }
}
