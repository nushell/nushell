//! Interactive last-result (`$ans` by default) helpers: truncation and AST detection.

use crate::{
    LAST_VARIABLE_ID, Span, Value,
    ast::{Block, Expr, Expression, Pipeline},
};

/// Truncate `value` so that [`Value::memory_size`] is at most `budget` bytes.
///
/// Returns the (possibly truncated) value and whether truncation occurred.
///
/// Strategy:
/// - **Lists of records (tables):** keep a leading prefix of *whole rows* only. Partial rows
///   (or `nothing` fillers) would break homogeneous-table rendering and collapse to
///   `{record N fields}` list view.
/// - **Other lists:** keep a leading prefix of whole items; for trailing scalars/strings/binary
///   only, a partial last item is allowed when it still fits cleanly.
/// - Records (standalone): fill fields left-to-right until the budget is exhausted
/// - Strings / binary / globs: keep a byte prefix
/// - Other scalar-ish values: keep whole if they fit, otherwise replace with `nothing`
pub fn truncate_value_to_budget(value: Value, budget: usize) -> (Value, bool) {
    if budget == 0 {
        return (Value::nothing(value.span()), true);
    }

    if value.memory_size() <= budget {
        return (value, false);
    }

    let span = value.span();
    match value {
        Value::List { vals, .. } => truncate_list(vals.into_owned(), budget, span),
        Value::Record { val, .. } => truncate_record(val.into_owned(), budget, span),
        Value::String { val, .. } => truncate_string(val, budget, span),
        Value::Binary { val, .. } => truncate_binary(val.into_owned(), budget, span),
        Value::Glob { val, .. } => {
            // Truncated globs become plain strings (prefix of the pattern).
            truncate_string(val, budget, span)
        }
        other => {
            // Cannot partially shrink; store nothing and mark truncated.
            drop(other);
            (Value::nothing(span), true)
        }
    }
}

fn truncate_list(vals: Vec<Value>, budget: usize, span: Span) -> (Value, bool) {
    // Base cost of the list shell; keep adding items while they fit.
    let base = Value::list(vec![], span).memory_size();
    if base > budget {
        return (Value::nothing(span), true);
    }

    let original_len = vals.len();
    // Table-like lists must keep whole rows so `table` still expands columns.
    let table_like = !vals.is_empty() && vals.iter().all(|v| matches!(v, Value::Record { .. }));

    let mut kept = Vec::new();
    let mut used = base;
    let mut truncated = false;

    for item in vals {
        let item_size = item.memory_size();
        if used.saturating_add(item_size) <= budget {
            used += item_size;
            kept.push(item);
            continue;
        }

        // Item does not fit whole.
        truncated = true;

        if table_like {
            // Drop this row and stop — do not emit a partial record or `nothing`.
            break;
        }

        // For non-table lists, allow a partial last scalar/string/binary only.
        // Never push `nothing` or empty placeholders that change list shape.
        let remaining = budget.saturating_sub(used);
        if remaining > 0 && can_partially_truncate_list_item(&item) {
            let (partial, _) = truncate_value_to_budget(item, remaining);
            if !matches!(partial, Value::Nothing { .. })
                && used.saturating_add(partial.memory_size()) <= budget
            {
                kept.push(partial);
            }
        }
        break;
    }

    if kept.len() < original_len {
        truncated = true;
    }

    let out = Value::list(kept, span);
    let still_over = out.memory_size() > budget;
    (out, truncated || still_over)
}

/// Whether a list item is a type we are willing to partially shrink as the last element.
fn can_partially_truncate_list_item(item: &Value) -> bool {
    matches!(
        item,
        Value::String { .. } | Value::Binary { .. } | Value::Glob { .. } | Value::List { .. }
    )
}

fn truncate_record(record: crate::Record, budget: usize, span: Span) -> (Value, bool) {
    let base = Value::record(crate::Record::new(), span).memory_size();
    if base > budget {
        return (Value::nothing(span), true);
    }

    let mut out = crate::Record::new();
    let mut used = base;
    let mut truncated = false;
    let original_len = record.len();

    for (key, val) in record {
        let key_cost = key.capacity();
        let remaining = budget.saturating_sub(used.saturating_add(key_cost));
        if remaining == 0 {
            truncated = true;
            break;
        }

        let (stored_val, val_trunc) = if val.memory_size() <= remaining {
            (val, false)
        } else {
            let (v, t) = truncate_value_to_budget(val, remaining);
            (v, t)
        };

        let entry_size = key_cost + stored_val.memory_size();
        if used.saturating_add(entry_size) > budget {
            truncated = true;
            break;
        }
        used += entry_size;
        truncated |= val_trunc;
        out.push(key, stored_val);
    }

    if out.len() < original_len {
        truncated = true;
    }

    (Value::record(out, span), truncated)
}

fn truncate_string(val: String, budget: usize, span: Span) -> (Value, bool) {
    // memory_size for string is size_of::<Value>() + capacity.
    let whole = Value::string(val, span);
    if whole.memory_size() <= budget {
        return (whole, false);
    }

    let val = match whole {
        Value::String { val, .. } => val,
        other => {
            // Only strings are passed here; fall back without panicking.
            drop(other);
            return (Value::nothing(span), true);
        }
    };

    if std::mem::size_of::<Value>() > budget {
        return (Value::nothing(span), true);
    }

    // Shrink content until Value::memory_size fits. capacity of a freshly built String equals len.
    let mut end = budget
        .saturating_sub(std::mem::size_of::<Value>())
        .min(val.len());
    while end > 0 && !val.is_char_boundary(end) {
        end -= 1;
    }

    loop {
        let prefix = val[..end].to_string();
        let out = Value::string(prefix, span);
        if out.memory_size() <= budget {
            return (out, true);
        }
        if end == 0 {
            return (Value::nothing(span), true);
        }
        end -= 1;
        while end > 0 && !val.is_char_boundary(end) {
            end -= 1;
        }
    }
}

fn truncate_binary(val: Vec<u8>, budget: usize, span: Span) -> (Value, bool) {
    let whole = Value::binary(val, span);
    if whole.memory_size() <= budget {
        return (whole, false);
    }

    let val = match whole {
        Value::Binary { val, .. } => val,
        other => {
            drop(other);
            return (Value::nothing(span), true);
        }
    };

    if std::mem::size_of::<Value>() > budget {
        return (Value::nothing(span), true);
    }

    let mut end = budget
        .saturating_sub(std::mem::size_of::<Value>())
        .min(val.len());
    loop {
        let out = Value::binary(val[..end].to_vec(), span);
        if out.memory_size() <= budget {
            return (out, true);
        }
        if end == 0 {
            return (Value::nothing(span), true);
        }
        end -= 1;
    }
}

/// Returns true when `block` is only a reference to the last-result variable or a
/// cell-path rooted at it (e.g. `$ans`, `$ans.last`, `$ans.exit_code`), optionally
/// wrapped in parentheses / a single-element pipeline.
///
/// Such expressions must not overwrite `$ans.last` when re-evaluated.
pub fn block_is_bare_last_result(block: &Block) -> bool {
    if block.pipelines.len() != 1 {
        return false;
    }
    pipeline_is_bare_last_result(&block.pipelines[0])
}

fn pipeline_is_bare_last_result(pipeline: &Pipeline) -> bool {
    if pipeline.elements.len() != 1 {
        return false;
    }
    let element = &pipeline.elements[0];
    if element.redirection.is_some() {
        return false;
    }
    expr_is_bare_last_result(&element.expr)
}

fn expr_is_bare_last_result(expr: &Expression) -> bool {
    match &expr.expr {
        Expr::Var(var_id) => *var_id == LAST_VARIABLE_ID,
        // Any cell path on `$ans` (including `$ans`, `$ans.last`, `$ans.last.0`, …).
        Expr::FullCellPath(path) => expr_is_bare_last_result(&path.head),
        // Parenthesized subexpression: `($ans)` / `($ans.last)`
        Expr::Block(block_id) | Expr::RowCondition(block_id) | Expr::Closure(block_id) => {
            // These shouldn't appear for simple paren groups; paren groups are usually Subexpression
            let _ = block_id;
            false
        }
        Expr::Subexpression(block_id) => {
            // Handled at call site if we have working set; treat conservatively as false here
            // unless we only have the Expression. Callers with EngineState should use the
            // overload below. Without block body we can't know — return false.
            let _ = block_id;
            false
        }
        _ => false,
    }
}

/// Like [`block_is_bare_last_result`] but expands subexpressions via `get_block`.
///
/// Uses a trait object so recursive subexpression walks do not monomorphize infinitely.
pub fn block_is_bare_last_result_with<'a>(
    block: &Block,
    get_block: &mut dyn FnMut(crate::BlockId) -> &'a Block,
) -> bool {
    if block.pipelines.len() != 1 {
        return false;
    }
    let pipeline = &block.pipelines[0];
    if pipeline.elements.len() != 1 {
        return false;
    }
    let element = &pipeline.elements[0];
    if element.redirection.is_some() {
        return false;
    }
    expr_is_bare_last_result_with(&element.expr, get_block)
}

fn expr_is_bare_last_result_with<'a>(
    expr: &Expression,
    get_block: &mut dyn FnMut(crate::BlockId) -> &'a Block,
) -> bool {
    match &expr.expr {
        Expr::Var(var_id) => *var_id == LAST_VARIABLE_ID,
        // Any cell path rooted at `$ans` skips re-storing `.last`.
        Expr::FullCellPath(path) => expr_is_bare_last_result_with(&path.head, get_block),
        Expr::Subexpression(block_id) => {
            let inner = get_block(*block_id);
            block_is_bare_last_result_with(inner, get_block)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record;

    #[test]
    fn under_budget_unchanged() {
        let v = Value::test_int(42);
        let size = v.memory_size();
        let (out, truncated) = truncate_value_to_budget(v.clone(), size);
        assert!(!truncated);
        assert_eq!(out, v);
        assert!(out.memory_size() <= size);
    }

    #[test]
    fn list_prefix_respects_budget() {
        let items: Vec<_> = (0..100).map(Value::test_int).collect();
        let list = Value::test_list(items);
        let full = list.memory_size();
        assert!(full > 0);

        // Budget that can only fit a few ints
        let one = Value::test_int(0).memory_size();
        let budget = Value::test_list(vec![]).memory_size() + one * 3;
        let (out, truncated) = truncate_value_to_budget(list, budget);
        assert!(truncated);
        assert!(out.memory_size() <= budget);
        match out {
            Value::List { vals, .. } => {
                assert!(vals.len() <= 3);
                assert!(
                    !vals.is_empty()
                        || budget < Value::test_list(vec![Value::test_int(0)]).memory_size()
                );
            }
            Value::Nothing { .. } => {
                // Acceptable only if budget was extremely tight
                assert!(budget < Value::test_list(vec![Value::test_int(0)]).memory_size());
            }
            other => panic!("unexpected truncated type: {other:?}"),
        }
    }

    #[test]
    fn string_prefix_respects_budget() {
        let s = "x".repeat(10_000);
        let val = Value::test_string(s);
        let budget = std::mem::size_of::<Value>() + 100;
        let (out, truncated) = truncate_value_to_budget(val, budget);
        assert!(truncated);
        assert!(out.memory_size() <= budget);
        if let Value::String { val, .. } = out {
            assert!(val.len() <= 100);
        }
    }

    #[test]
    fn binary_prefix_respects_budget() {
        let val = Value::test_binary(vec![0u8; 10_000]);
        let budget = std::mem::size_of::<Value>() + 64;
        let (out, truncated) = truncate_value_to_budget(val, budget);
        assert!(truncated);
        assert!(out.memory_size() <= budget);
        if let Value::Binary { val, .. } = out {
            assert!(val.len() <= 64);
        }
    }

    #[test]
    fn zero_budget_yields_nothing() {
        let v = Value::test_string("hello");
        let (out, truncated) = truncate_value_to_budget(v, 0);
        assert!(truncated);
        assert!(matches!(out, Value::Nothing { .. }));
    }

    #[test]
    fn table_like_list_keeps_whole_records_only() {
        // ls-style rows: homogeneous records must stay full so `table` expands columns.
        let row = |name: &str| {
            Value::test_record(record! {
                "name" => Value::test_string(name),
                "type" => Value::test_string("file"),
                "size" => Value::test_int(1),
                "modified" => Value::test_string("now"),
            })
        };
        let rows: Vec<_> = (0..50).map(|i| row(&format!("f{i}.txt"))).collect();
        let list = Value::test_list(rows);
        let one = row("x").memory_size();
        let budget = Value::test_list(vec![]).memory_size() + one * 3 + one / 2;

        let (out, truncated) = truncate_value_to_budget(list, budget);
        assert!(truncated);
        assert!(out.memory_size() <= budget);

        let Value::List { vals, .. } = out else {
            panic!("expected list, got {out:?}");
        };
        assert!(vals.len() <= 3);
        assert!(!vals.is_empty());
        // Every kept row must be a full record with the same columns (no nothing fillers).
        for v in &vals {
            match v {
                Value::Record { val, .. } => {
                    assert_eq!(val.len(), 4, "row must keep all columns for table display");
                    assert!(val.get("name").is_some());
                    assert!(val.get("type").is_some());
                    assert!(val.get("size").is_some());
                    assert!(val.get("modified").is_some());
                }
                other => panic!("expected full record row, got {other:?}"),
            }
        }
    }

    #[test]
    fn record_fields_respect_budget() {
        let rec = Value::test_record(record! {
            "a" => Value::test_string("x".repeat(5000)),
            "b" => Value::test_string("y".repeat(5000)),
        });
        let budget = rec.memory_size() / 2;
        let (out, truncated) = truncate_value_to_budget(rec, budget);
        assert!(truncated);
        assert!(out.memory_size() <= budget);
    }
}
