use crate::{
    database::values::dsl::{ExprDb, SelectDb},
    SQLiteDatabase,
};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape,
};
use sqlparser::ast::{Ident, SelectItem, SetExpr, Statement, TableAlias, TableFactor};

#[derive(Clone)]
pub struct AliasExpr;

impl Command for AliasExpr {
    fn name(&self) -> &str {
        "db as"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("alias", SyntaxShape::String, "alias name")
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
        "Creates an alias for a column selection"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates an alias for a column selection",
            example: "db col name_a | db as new_a",
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "alias", "column"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let alias: String = call.req(engine_state, stack, 0)?;
        let value = input.into_value(call.head);

        if let Ok(expr) = ExprDb::try_from_value(&value) {
            alias_selection(expr.into_native().into(), alias, call)
        } else if let Ok(select) = SelectDb::try_from_value(&value) {
            alias_selection(select, alias, call)
        } else if let Ok(db) = SQLiteDatabase::try_from_value(value.clone()) {
            alias_db(db, alias, call)
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

fn alias_selection(
    select: SelectDb,
    alias: String,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let select = match select.into_native() {
        SelectItem::UnnamedExpr(expr) => SelectItem::ExprWithAlias {
            expr,
            alias: Ident {
                value: alias,
                quote_style: None,
            },
        },
        SelectItem::ExprWithAlias { expr, .. } => SelectItem::ExprWithAlias {
            expr,
            alias: Ident {
                value: alias,
                quote_style: None,
            },
        },
        select => select,
    };

    let select: SelectDb = select.into();
    Ok(select.into_value(call.head).into_pipeline_data())
}

fn alias_db(
    mut db: SQLiteDatabase,
    new_alias: String,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    match db.statement.as_mut() {
        None => Err(ShellError::GenericError(
            "Error creating alias".into(),
            "there is no statement defined yet".into(),
            Some(call.head),
            None,
            Vec::new(),
        )),
        Some(statement) => match statement {
            Statement::Query(query) => match &mut query.body {
                SetExpr::Select(select) => {
                    select.as_mut().from.iter_mut().for_each(|table| {
                        let new_alias = Some(TableAlias {
                            name: Ident {
                                value: new_alias.clone(),
                                quote_style: None,
                            },
                            columns: Vec::new(),
                        });

                        if let TableFactor::Table { ref mut alias, .. } = table.relation {
                            *alias = new_alias;
                        } else if let TableFactor::Derived { ref mut alias, .. } = table.relation {
                            *alias = new_alias;
                        } else if let TableFactor::TableFunction { ref mut alias, .. } =
                            table.relation
                        {
                            *alias = new_alias;
                        }
                    });

                    Ok(db.into_value(call.head).into_pipeline_data())
                }
                _ => Err(ShellError::GenericError(
                    "Error creating alias".into(),
                    "Query has no select from defined".into(),
                    Some(call.head),
                    None,
                    Vec::new(),
                )),
            },
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
    }
}
