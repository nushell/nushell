use super::{super::SQLiteDatabase, conversions::value_into_table_factor};
use crate::database::values::{definitions::ConnectionDb, dsl::ExprDb};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use sqlparser::ast::{
    Ident, Join, JoinConstraint, JoinOperator, Select, SetExpr, Statement, TableAlias,
};

#[derive(Clone)]
pub struct JoinDb;

impl Command for JoinDb {
    fn name(&self) -> &str {
        "db join"
    }

    fn usage(&self) -> &str {
        "Joins with another table or derived table. Default join type is inner"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "table",
                SyntaxShape::Any,
                "table or derived table to join on",
            )
            .required("on", SyntaxShape::Any, "expression to join tables")
            .named(
                "as",
                SyntaxShape::String,
                "Alias for the selected join",
                Some('a'),
            )
            .switch("left", "left outer join", Some('l'))
            .switch("right", "right outer join", Some('r'))
            .switch("outer", "full outer join", Some('o'))
            .switch("cross", "cross join", Some('c'))
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "join"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "",
            example: "",
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
        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;

        db.statement = match db.statement {
            Some(statement) => Some(modify_statement(
                &db.connection,
                statement,
                engine_state,
                stack,
                call,
            )?),
            None => {
                return Err(ShellError::GenericError(
                    "Error creating join".into(),
                    "there is no statement defined yet".into(),
                    Some(call.head),
                    None,
                    Vec::new(),
                ))
            }
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

fn modify_statement(
    connection: &ConnectionDb,
    mut statement: Statement,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Statement, ShellError> {
    match statement {
        Statement::Query(ref mut query) => {
            match &mut query.body {
                SetExpr::Select(ref mut select) => {
                    modify_from(connection, select, engine_state, stack, call)?
                }
                s => {
                    return Err(ShellError::GenericError(
                        "Connection doesnt define a select".into(),
                        format!("Expected a connection with select. Got {}", s),
                        Some(call.head),
                        None,
                        Vec::new(),
                    ))
                }
            };

            Ok(statement)
        }
        s => Err(ShellError::GenericError(
            "Connection doesnt define a query".into(),
            format!("Expected a connection with query. Got {}", s),
            Some(call.head),
            None,
            Vec::new(),
        )),
    }
}

fn modify_from(
    connection: &ConnectionDb,
    select: &mut Select,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<(), ShellError> {
    match select.from.last_mut() {
        Some(table) => {
            let alias = call
                .get_flag::<String>(engine_state, stack, "as")?
                .map(|alias| TableAlias {
                    name: Ident {
                        value: alias,
                        quote_style: None,
                    },
                    columns: Vec::new(),
                });

            let join_table: Value = call.req(engine_state, stack, 0)?;
            let table_factor = value_into_table_factor(join_table, connection, alias)?;

            let on_expr: Value = call.req(engine_state, stack, 1)?;
            let on_expr = ExprDb::try_from_value(&on_expr)?;

            let join_on = if call.has_flag("left") {
                JoinOperator::LeftOuter(JoinConstraint::On(on_expr.into_native()))
            } else if call.has_flag("right") {
                JoinOperator::RightOuter(JoinConstraint::On(on_expr.into_native()))
            } else if call.has_flag("outer") {
                JoinOperator::FullOuter(JoinConstraint::On(on_expr.into_native()))
            } else {
                JoinOperator::Inner(JoinConstraint::On(on_expr.into_native()))
            };

            let join = Join {
                relation: table_factor,
                join_operator: join_on,
            };

            table.joins.push(join);

            Ok(())
        }
        None => Err(ShellError::GenericError(
            "Connection without table defined".into(),
            "Expected a table defined".into(),
            Some(call.head),
            None,
            Vec::new(),
        )),
    }
}
