use crate::database::values::dsl::ExprDb;

use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};
use sqlparser::ast::{BinaryOperator, Expr, Query, Select, SetExpr};

#[derive(Clone)]
pub struct OrDb;

impl Command for OrDb {
    fn name(&self) -> &str {
        "db or"
    }

    fn usage(&self) -> &str {
        "Includes an OR clause for a query or expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("where", SyntaxShape::Any, "Where expression on the table")
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "where"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "selects a column from a database with a where clause",
                example: r#"db open db.mysql 
    | db select a 
    | db from table_1 
    | db where ((db col a) > 1) 
    | db or ((db col b) == 1) 
    | db describe"#,
                result: None,
            },
            Example {
                description: "Creates a nested where clause",
                example: r#"db open db.mysql 
    | db select a 
    | db from table_1 
    | db where ((db col a) > 1 | db or ((db col a) < 10)) 
    | db describe"#,
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value: Value = call.req(engine_state, stack, 0)?;
        let expr = ExprDb::try_from_value(value)?.into_native();

        let value = input.into_value(call.head);
        if let Ok(expression) = ExprDb::try_from_value(value.clone()) {
            let expression = Expr::BinaryOp {
                left: Box::new(expression.into_native()),
                op: BinaryOperator::Or,
                right: Box::new(expr),
            };

            let expression: ExprDb = Expr::Nested(Box::new(expression)).into();

            Ok(expression.into_value(call.head).into_pipeline_data())
        } else if let Ok(mut db) = SQLiteDatabase::try_from_value(value.clone()) {
            db.query = match db.query {
                Some(query) => Some(modify_query(query, expr, call.head)?),
                None => {
                    return Err(ShellError::GenericError(
                        "Connection without query".into(),
                        "Missing query in the connection".into(),
                        Some(call.head),
                        None,
                        Vec::new(),
                    ))
                }
            };

            Ok(db.into_value(call.head).into_pipeline_data())
        } else {
            Err(ShellError::CantConvert(
                "expression or query".into(),
                value.get_type().to_string(),
                value.span()?,
                None,
            ))
        }
    }
}

fn modify_query(mut query: Query, expression: Expr, span: Span) -> Result<Query, ShellError> {
    query.body = match query.body {
        SetExpr::Select(select) => Ok(SetExpr::Select(modify_select(select, expression, span)?)),
        _ => Err(ShellError::GenericError(
            "Query without a select".into(),
            "Missing a WHERE clause before an OR clause".into(),
            Some(span),
            None,
            Vec::new(),
        )),
    }?;

    Ok(query)
}

fn modify_select(
    mut select: Box<Select>,
    expression: Expr,
    span: Span,
) -> Result<Box<Select>, ShellError> {
    let new_expression = match &select.selection {
        Some(expr) => Ok(Expr::BinaryOp {
            left: Box::new(expr.clone()),
            op: BinaryOperator::Or,
            right: Box::new(expression),
        }),
        None => Err(ShellError::GenericError(
            "Query without a select".into(),
            "Missing a WHERE clause before an OR clause".into(),
            Some(span),
            None,
            Vec::new(),
        )),
    }?;

    select.as_mut().selection = Some(new_expression);
    Ok(select)
}
