use crate::database::values::sqlite::SQLiteQueryBuilder;
use nu_protocol::{PipelineData, ShellError, Span, Value};

/// A lazy query plan that can be composed and executed by filter commands.
///
/// This is the deepened seam between filter commands (which want to push down
/// operations like limit, select, count) and database backends (which implement
/// the actual query execution). Each variant wraps a concrete query builder.
///
/// Callers interact with this enum through its methods instead of downcasting
/// to a concrete type. Adding a new backend means adding a variant here;
/// filter commands do not change.
pub enum QueryPlan {
    /// SQLite-based lazy query (via `SQLiteQueryBuilder`).
    Sqlite(SQLiteQueryBuilder),
}

impl QueryPlan {
    /// Try to extract a `QueryPlan` from a `&dyn Any` reference.
    ///
    /// This is the single place where `downcast_ref` happens. All filter
    /// commands call this instead of importing and downcasting to a concrete
    /// query builder type.
    pub fn try_from_any(val: &dyn std::any::Any) -> Option<Self> {
        val.downcast_ref::<SQLiteQueryBuilder>()
            .map(|b| Self::Sqlite(b.clone()))
    }

    /// Apply a LIMIT to the query plan.
    pub fn with_limit(self, limit: i64) -> Self {
        match self {
            Self::Sqlite(b) => Self::Sqlite(b.with_limit(limit)),
        }
    }

    /// Apply an OFFSET to the query plan.
    pub fn with_offset(self, offset: i64) -> Self {
        match self {
            Self::Sqlite(b) => Self::Sqlite(b.with_offset(offset)),
        }
    }

    /// Apply a DISTINCT to the query plan.
    pub fn with_distinct(self) -> Self {
        match self {
            Self::Sqlite(b) => Self::Sqlite(b.with_distinct()),
        }
    }

    /// Apply an ORDER BY to the query plan.
    pub fn with_order_by(self, order_by: String) -> Self {
        match self {
            Self::Sqlite(b) => Self::Sqlite(b.with_order_by(order_by)),
        }
    }

    /// Project the output to a subset of columns.
    ///
    /// Returns `None` if the projection cannot be expressed (e.g. complex
    /// column expressions), allowing callers to fall back to in-memory
    /// processing.
    pub fn project_output_columns(&self, columns: &[String]) -> Option<Self> {
        match self {
            Self::Sqlite(b) => b.project_output_columns(columns).map(Self::Sqlite),
        }
    }

    /// Execute the query and return the result as `PipelineData`.
    pub fn execute(&self, call_span: Span) -> Result<PipelineData, ShellError> {
        match self {
            Self::Sqlite(b) => b.execute(call_span),
        }
    }

    /// Count the number of rows without fetching them.
    pub fn count(&self, call_span: Span) -> Result<i64, ShellError> {
        match self {
            Self::Sqlite(b) => b.count(call_span),
        }
    }

    /// Human-readable type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => "lazy query",
        }
    }

    /// Convert back into a `Value::Custom` for pipeline propagation.
    pub fn into_value(self, span: Span) -> Value {
        match self {
            Self::Sqlite(b) => Value::custom(Box::new(b), span),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::database::values::sqlite::SQLiteDatabase;
    use nu_protocol::Signals;
    use std::path::Path;

    fn sample_builder(table: &str) -> SQLiteQueryBuilder {
        SQLiteQueryBuilder::new(
            Path::new(":memory:").to_path_buf(),
            table.to_string(),
            Signals::empty(),
        )
    }

    #[test]
    fn try_from_any_accepts_sqlite_query_builder() {
        let builder = sample_builder("test");
        let plan = QueryPlan::try_from_any(&builder as &dyn std::any::Any);
        assert!(matches!(plan, Some(QueryPlan::Sqlite(_))));
    }

    #[test]
    fn try_from_any_rejects_other_custom_values() {
        let db = SQLiteDatabase::new(Path::new(":memory:"), Signals::empty());
        let plan = QueryPlan::try_from_any(&db as &dyn std::any::Any);
        assert!(plan.is_none());
    }

    #[test]
    fn try_from_any_rejects_non_custom_values() {
        let val = 42i64;
        let plan = QueryPlan::try_from_any(&val as &dyn std::any::Any);
        assert!(plan.is_none());
    }

    #[test]
    fn with_limit_delegates() {
        let plan = QueryPlan::Sqlite(sample_builder("t"));
        let limited = plan.with_limit(5);
        assert!(matches!(limited, QueryPlan::Sqlite(_)));
        // Verify the limit took effect by checking the generated SQL
        let sql = match &limited {
            QueryPlan::Sqlite(b) => b.build_sql(),
        };
        assert!(
            sql.contains("LIMIT 5"),
            "SQL should contain LIMIT 5, got: {sql}"
        );
    }

    #[test]
    fn with_offset_delegates() {
        let plan = QueryPlan::Sqlite(sample_builder("t"));
        let offset = plan.with_offset(10);
        let sql = match &offset {
            QueryPlan::Sqlite(b) => b.build_sql(),
        };
        assert!(
            sql.contains("OFFSET 10"),
            "SQL should contain OFFSET 10, got: {sql}"
        );
    }

    #[test]
    fn with_distinct_delegates() {
        let plan = QueryPlan::Sqlite(sample_builder("t"));
        let distinct = plan.with_distinct();
        let sql = match &distinct {
            QueryPlan::Sqlite(b) => b.build_sql(),
        };
        assert!(
            sql.starts_with("SELECT DISTINCT"),
            "SQL should start with SELECT DISTINCT, got: {sql}"
        );
    }

    #[test]
    fn with_order_by_delegates() {
        let plan = QueryPlan::Sqlite(sample_builder("t"));
        let ordered = plan.with_order_by("id DESC".to_string());
        let sql = match &ordered {
            QueryPlan::Sqlite(b) => b.build_sql(),
        };
        assert!(
            sql.contains("ORDER BY id DESC"),
            "SQL should contain ORDER BY id DESC, got: {sql}"
        );
    }

    #[test]
    fn type_name_returns_expected() {
        let plan = QueryPlan::Sqlite(sample_builder("t"));
        assert_eq!(plan.type_name(), "lazy query");
    }

    #[test]
    fn into_value_round_trip_produces_sqlite_query_builder() {
        let plan = QueryPlan::Sqlite(sample_builder("roundtrip"));
        let span = Span::test_data();
        let value = plan.into_value(span);

        // It should be a Value::Custom
        assert!(matches!(&value, Value::Custom { .. }));

        // The inner CustomValue should be a SQLiteQueryBuilder
        if let Value::Custom { val, .. } = &value {
            let retrieved = val.as_any().downcast_ref::<SQLiteQueryBuilder>();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().table_name, "roundtrip");
        }
    }
}
