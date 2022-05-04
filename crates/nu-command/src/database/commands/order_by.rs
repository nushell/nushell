use crate::database::values::dsl::ExprDb;

use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use sqlparser::ast::{OrderByExpr, Statement};

#[derive(Clone)]
pub struct OrderByDb;

impl Command for OrderByDb {
    fn name(&self) -> &str {
        "db order-by"
    }

    fn usage(&self) -> &str {
        "Orders by query"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("ascending", "Order by ascending values", Some('a'))
            .switch("nulls_first", "Show nulls first in order", Some('n'))
            .rest(
                "select",
                SyntaxShape::Any,
                "Select expression(s) on the table",
            )
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "select"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "orders query by a column",
            example: r#"db open db.mysql 
    | db from table_a 
    | db select a 
    | db order-by a 
    | db describe"#,
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let asc = call.has_flag("ascending");
        let nulls_first = call.has_flag("nulls_first");

        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::List {
            vals,
            span: call.head,
        };
        let expressions = ExprDb::extract_exprs(value)?;

        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        match db.statement {
            Some(ref mut statement) => match statement {
                Statement::Query(ref mut query) => {
                    let mut order_expr: Vec<OrderByExpr> = expressions
                        .into_iter()
                        .map(|expr| OrderByExpr {
                            expr,
                            asc: if asc { Some(asc) } else { None },
                            nulls_first: if nulls_first { Some(nulls_first) } else { None },
                        })
                        .collect();

                    query.order_by.append(&mut order_expr);
                }
                s => {
                    return Err(ShellError::GenericError(
                        "Connection doesnt define a query".into(),
                        format!("Expected a connection with query. Got {}", s),
                        Some(call.head),
                        None,
                        Vec::new(),
                    ))
                }
            },
            None => {
                return Err(ShellError::GenericError(
                    "Connection without statement".into(),
                    "The connection needs a statement defined".into(),
                    Some(call.head),
                    None,
                    Vec::new(),
                ))
            }
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}
