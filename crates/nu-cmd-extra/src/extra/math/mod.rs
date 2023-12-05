mod cos;
mod cosh;
mod sin;
mod sinh;
mod tan;
mod tanh;

mod exp;
mod ln;

mod arccos;
mod arccosh;
mod arcsin;
mod arcsinh;
mod arctan;
mod arctanh;

pub use cos::SubCommand as MathCos;
pub use cosh::SubCommand as MathCosH;
pub use sin::SubCommand as MathSin;
pub use sinh::SubCommand as MathSinH;
pub use tan::SubCommand as MathTan;
pub use tanh::SubCommand as MathTanH;

pub use exp::SubCommand as MathExp;
pub use ln::SubCommand as MathLn;

pub use arccos::SubCommand as MathArcCos;
pub use arccosh::SubCommand as MathArcCosH;
pub use arcsin::SubCommand as MathArcSin;
pub use arcsinh::SubCommand as MathArcSinH;
pub use arctan::SubCommand as MathArcTan;
pub use arctanh::SubCommand as MathArcTanH;
