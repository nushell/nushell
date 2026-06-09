mod abs;
mod bitwise_and;
mod bitwise_count_ones;
mod bitwise_count_zeros;
mod bitwise_leading_ones;
mod bitwise_leading_zeros;
mod bitwise_or;
mod bitwise_trailing_ones;
mod bitwise_trailing_zeros;
mod bitwise_xor;
mod cos;
mod dot;
mod exp;
mod log;
mod log1p;
mod sign;
mod sin;
mod sqrt;
mod stub;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub(crate) fn commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(stub::MathCmd),
        Box::new(abs::ExprMathAbs),
        Box::new(bitwise_and::ExprMathBitwiseAnd),
        Box::new(bitwise_count_ones::ExprMathBitwiseCountOnes),
        Box::new(bitwise_count_zeros::ExprMathBitwiseCountZeros),
        Box::new(bitwise_leading_ones::ExprMathBitwiseLeadingOnes),
        Box::new(bitwise_leading_zeros::ExprMathBitwiseLeadingZeros),
        Box::new(bitwise_or::ExprMathBitwiseOr),
        Box::new(bitwise_trailing_ones::ExprMathBitwiseTrailingOnes),
        Box::new(bitwise_trailing_zeros::ExprMathBitwiseTrailingZeros),
        Box::new(bitwise_xor::ExprMathBitwiseXor),
        Box::new(cos::ExprMathCos),
        Box::new(dot::ExprMathDot),
        Box::new(exp::ExprMathExp),
        Box::new(log::ExprMathLog),
        Box::new(log1p::ExprMathLog1p),
        Box::new(sign::ExprMathSign),
        Box::new(sin::ExprMathSin),
        Box::new(sqrt::ExprMathSqrt),
    ]
}
