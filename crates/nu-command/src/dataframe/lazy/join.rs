use crate::dataframe::values::{NuExpression, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::{Expr, JoinType};

#[derive(Clone)]
pub struct LazyJoin;

impl Command for LazyJoin {
    fn name(&self) -> &str {
        "dfr join"
    }

    fn usage(&self) -> &str {
        "Joins a lazy frame with other lazy frame"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("other", SyntaxShape::Any, "LazyFrame to join with")
            .required("left_on", SyntaxShape::Any, "Left column(s) to join on")
            .required("right_on", SyntaxShape::Any, "Right column(s) to join on")
            .switch(
                "inner",
                "inner joing between lazyframes (default)",
                Some('i'),
            )
            .switch("left", "left join between lazyframes", Some('l'))
            .switch("outer", "outer join between lazyframes", Some('o'))
            .switch("cross", "cross join between lazyframes", Some('c'))
            .named(
                "suffix",
                SyntaxShape::String,
                "Suffix to use on columns with same name",
                Some('s'),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Join two lazy dataframes",
                example: r#"let df_a = ([[a b c];[1 "a" 0] [2 "b" 1] [1 "c" 2] [1 "c" 3]] | dfr to-lazy);
    let df_b = ([["foo" "bar" "ham"];[1 "a" "let"] [1 "c" "var"] [1 "c" "const"]] | dfr to-lazy);
    $df_a | dfr join $df_b a foo | dfr collect"#,
                result: None,
            },
            Example {
                description: "Join one eager dataframe with a lazy dataframe",
                example: r#"let df_a = ([[a b c];[1 "a" 0] [2 "b" 1] [1 "c" 2] [1 "c" 3]] | dfr to-df);
    let df_b = ([["foo" "bar" "ham"];[1 "a" "let"] [1 "c" "var"] [1 "c" "const"]] | dfr to-lazy);
    $df_a | dfr join $df_b a foo"#,
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
        let left = call.has_flag("left");
        let outer = call.has_flag("outer");
        let cross = call.has_flag("cross");

        let how = if left {
            JoinType::Left
        } else if outer {
            JoinType::Outer
        } else if cross {
            JoinType::Cross
        } else {
            JoinType::Inner
        };

        let other: Value = call.req(engine_state, stack, 0)?;
        let (other, _) = NuLazyFrame::maybe_is_eager(other)?;
        let other = other.into_polars();

        let left_on: Value = call.req(engine_state, stack, 1)?;
        let left_on = NuExpression::extract_exprs(left_on)?;

        let right_on: Value = call.req(engine_state, stack, 2)?;
        let right_on = NuExpression::extract_exprs(right_on)?;

        if left_on.len() != right_on.len() {
            let right_on: Value = call.req(engine_state, stack, 2)?;
            return Err(ShellError::IncompatibleParametersSingle(
                "The right column list has a different size to the left column list".into(),
                right_on.span()?,
            ));
        }

        // Checking that both list of expressions are made out of col expressions or strings
        for (index, list) in &[(1usize, &left_on), (2, &left_on)] {
            if list.iter().any(|expr| !matches!(expr, Expr::Column(..))) {
                let value: Value = call.req(engine_state, stack, *index)?;
                return Err(ShellError::IncompatibleParametersSingle(
                    "Expected only a string, col expressions or list of strings".into(),
                    value.span()?,
                ));
            }
        }

        let suffix: Option<String> = call.get_flag(engine_state, stack, "suffix")?;
        let suffix = suffix.unwrap_or_else(|| "_x".into());

        let value = input.into_value(call.head);
        let (lazy, from_eager) = NuLazyFrame::maybe_is_eager(value)?;
        let lazy = lazy.into_polars();

        let lazy: NuLazyFrame = lazy
            .join_builder()
            .with(other)
            .left_on(left_on)
            .right_on(right_on)
            .how(how)
            .force_parallel(true)
            .suffix(suffix)
            .finish()
            .into();

        let res = if from_eager {
            lazy.collect(call.head)?.into_value(call.head)
        } else {
            lazy.into_value(call.head)
        };

        Ok(PipelineData::Value(res, None))
    }
}
