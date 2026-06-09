mod math_abs;
mod math_bitwise_and;
mod math_bitwise_count_ones;
mod math_bitwise_count_zeros;
mod math_bitwise_leading_ones;
mod math_bitwise_leading_zeros;
mod math_bitwise_or;
mod math_bitwise_trailing_ones;
mod math_bitwise_trailing_zeros;
mod math_bitwise_xor;
mod math_cos;
mod math_dot;
mod math_exp;
mod math_log;
mod math_log1p;
mod math_sign;
mod math_sin;
mod math_sqrt;
mod math_stub;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub(crate) fn math_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(math_stub::MathCmd),
        Box::new(math_abs::ExprMathAbs),
        Box::new(math_bitwise_and::ExprMathBitwiseAnd),
        Box::new(math_bitwise_count_ones::ExprMathBitwiseCountOnes),
        Box::new(math_bitwise_count_zeros::ExprMathBitwiseCountZeros),
        Box::new(math_bitwise_leading_ones::ExprMathBitwiseLeadingOnes),
        Box::new(math_bitwise_leading_zeros::ExprMathBitwiseLeadingZeros),
        Box::new(math_bitwise_or::ExprMathBitwiseOr),
        Box::new(math_bitwise_trailing_ones::ExprMathBitwiseTrailingOnes),
        Box::new(math_bitwise_trailing_zeros::ExprMathBitwiseTrailingZeros),
        Box::new(math_bitwise_xor::ExprMathBitwiseXor),
        Box::new(math_cos::ExprMathCos),
        Box::new(math_dot::ExprMathDot),
        Box::new(math_exp::ExprMathExp),
        Box::new(math_log::ExprMathLog),
        Box::new(math_log1p::ExprMathLog1p),
        Box::new(math_sign::ExprMathSign),
        Box::new(math_sin::ExprMathSin),
        Box::new(math_sqrt::ExprMathSqrt),
    ]
}
