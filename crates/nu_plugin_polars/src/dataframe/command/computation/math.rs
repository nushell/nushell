use crate::{PolarsPlugin, values::CustomValueSupport};

use crate::values::{
    NuDataFrame, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value,
};
use polars::prelude::{Expr, Literal, df};

enum FunctionType {
    Abs,
    Cos,
    Dot,
    Exp,
    Log,
    Log1p,
    Sign,
    Sin,
    Sqrt,
}

impl FunctionType {
    fn from_str(func_type: &str, span: Span) -> Result<Self, ShellError> {
        match func_type {
            "abs" => Ok(Self::Abs),
            "cos" => Ok(Self::Cos),
            "dot" => Ok(Self::Dot),
            "exp" => Ok(Self::Exp),
            "log" => Ok(Self::Log),
            "log1p" => Ok(Self::Log1p),
            "sign" => Ok(Self::Sign),
            "sin" => Ok(Self::Sin),
            "sqrt" => Ok(Self::Sqrt),
            _ => Err(ShellError::GenericError {
                error: "Invalid function name".into(),
                msg: "".into(),
                span: Some(span),
                help: Some("See description for accepted functions".into()),
                inner: vec![],
            }),
        }
    }

    #[allow(dead_code)]
    fn to_str(&self) -> &'static str {
        match self {
            FunctionType::Abs => "abs",
            FunctionType::Cos => "cos",
            FunctionType::Dot => "dot",
            FunctionType::Exp => "exp",
            FunctionType::Log => "log",
            FunctionType::Log1p => "log1p",
            FunctionType::Sign => "sign",
            FunctionType::Sin => "sin",
            FunctionType::Sqrt => "sqrt",
        }
    }
}

#[derive(Clone)]
pub struct ExprMath;

impl PluginCommand for ExprMath {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars math"
    }

    fn description(&self) -> &str {
        "Collection of math functions to be applied on one or more column expressions"
    }

    fn extra_description(&self) -> &str {
        r#"This is an incomplete implementation of the available functions listed here: https://docs.pola.rs/api/python/stable/reference/expressions/computation.html.

        The following functions are currently available:
        - abs
        - cos
        - dot <expression>
        - exp
        - log <base; default e>
        - log1p
        - sign
        - sin
        - sqrt
        "#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "type",
                SyntaxShape::String,
                "Function name. See extra description for full list of accepted values",
            )
            .rest(
                "args",
                SyntaxShape::Any,
                "Extra arguments required by some functions",
            )
            .input_output_types(vec![(
                PolarsPluginType::NuExpression.into(),
                PolarsPluginType::NuExpression.into(),
            )])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Apply function to column expression",
            example: "[[a]; [0] [-1] [2] [-3] [4]]
                    | polars into-df
                    | polars select [
                        (polars col a | polars math abs | polars as a_abs)
                        (polars col a | polars math sign | polars as a_sign)
                        (polars col a | polars math exp | polars as a_exp)]
                    | polars collect",
            result: Some(
                NuDataFrame::from(
                    df!(
                        "a_abs" => [0, 1, 2, 3, 4],
                        "a_sign" => [0, -1, 1, -1, 1],
                        "a_exp" => [1.000, 0.36787944117144233, 7.38905609893065, 0.049787068367863944, 54.598150033144236],
                    )
                    .expect("simple df for test should not fail"),
                )
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Specify arguments for select functions. See description for more information.",
            example: "[[a]; [0] [1] [2] [4] [8] [16]]
                    | polars into-df
                    | polars select [
                        (polars col a | polars math log 2 | polars as a_base2)]
                    | polars collect",
            result: Some(
                NuDataFrame::from(
                    df!(
                        "a_base2" => [f64::NEG_INFINITY, 0.0, 1.0, 2.0, 3.0, 4.0],
                    )
                    .expect("simple df for test should not fail"),
                )
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Specify arguments for select functions. See description for more information.",
            example: "[[a b]; [0 0] [1 1] [2 2] [3 3] [4 4] [5 5]]
                    | polars into-df
                    | polars select [
                        (polars col a | polars math dot (polars col b) | polars as ab)]
                    | polars collect",
            result: Some(
                NuDataFrame::from(
                    df!(
                        "ab" => [55.0],
                    )
                    .expect("simple df for test should not fail"),
                )
                .into_value(Span::test_data()),
            ),
        }
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let value = input.into_value(call.head)?;
        let func_type: Spanned<String> = call.req(0)?;
        let func_type = FunctionType::from_str(&func_type.item, func_type.span)?;

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuExpression(expr) => {
                command_expr(plugin, engine, call, func_type, expr)
            }
            _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    func_type: FunctionType,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let res = expr.into_polars();

    let res: NuExpression = match func_type {
        FunctionType::Abs => res.abs(),
        FunctionType::Cos => res.cos(),
        FunctionType::Dot => {
            let expr: Expr = match call.rest::<Value>(1)?.first() {
                None => {
                    return Err(ShellError::GenericError {
                        error: "Second expression to compute dot product with must be provided"
                            .into(),
                        msg: "".into(),
                        span: Some(call.head),
                        help: None,
                        inner: vec![],
                    });
                }
                Some(value) => NuExpression::try_from_value(plugin, value)?.into_polars(),
            };
            res.dot(expr)
        }
        FunctionType::Exp => res.exp(),
        FunctionType::Log => {
            let base: Expr = match call.rest::<Value>(1)?.first() {
                // default natural log
                None => std::f64::consts::E.lit(),
                Some(value) => NuExpression::try_from_value(plugin, value)?.into_polars(),
            };

            res.log(base)
        }
        FunctionType::Log1p => res.log1p(),
        FunctionType::Sign => res.sign(),
        FunctionType::Sin => res.sin(),
        FunctionType::Sqrt => res.sqrt(),
    }
    .into();

    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprMath)
    }
}
