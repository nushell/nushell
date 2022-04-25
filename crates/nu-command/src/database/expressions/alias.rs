use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape,
};
use sqlparser::ast::{Ident, SelectItem};

use crate::database::values::SelectDb;

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
            description: "Creates an alias for a a column selection",
            example: "db col name_a | db as new_a",
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "column", "expression"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let alias: String = call.req(engine_state, stack, 0)?;
        let select = SelectDb::try_from_pipeline(input, call.head)?;

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
}
