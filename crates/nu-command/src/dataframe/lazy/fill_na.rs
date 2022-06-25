use crate::dataframe::values::{NuExpression, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct LazyFillNA;

impl Command for LazyFillNA {
    fn name(&self) -> &str {
        "fill-na"
    }

    fn usage(&self) -> &str {
        "Replaces NA values with the given expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "fill",
                SyntaxShape::Any,
                "Expression to use to fill the NAN values",
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("lazyframe".into()))
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
        let fill: Value = call.req(engine_state, stack, 0)?;
        let value = input.into_value(call.head);

        if NuExpression::can_downcast(&value) {
            let expr = NuExpression::try_from_value(value)?;
            let fill = NuExpression::try_from_value(fill)?.into_polars();
            let expr: NuExpression = expr.into_polars().fill_nan(fill).into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        } else {
            let lazy = NuLazyFrame::try_from_value(value)?;
            let expr = NuExpression::try_from_value(fill)?.into_polars();
            let lazy = NuLazyFrame::new(lazy.from_eager, lazy.into_polars().fill_nan(expr));

            Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
        }
    }
}
