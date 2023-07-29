mod cos;
mod cosh;
mod sin;
mod sinh;
mod tan;
mod tanh;

mod egamma;
mod euler;
mod exp;
mod ln;
mod phi;
mod pi;
mod tau;

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

pub use egamma::SubCommand as MathEulerGamma;
pub use euler::SubCommand as MathEuler;
pub use exp::SubCommand as MathExp;
pub use ln::SubCommand as MathLn;
pub use phi::SubCommand as MathPhi;
pub use pi::SubCommand as MathPi;
pub use tau::SubCommand as MathTau;

pub use arccos::SubCommand as MathArcCos;
pub use arccosh::SubCommand as MathArcCosH;
pub use arcsin::SubCommand as MathArcSin;
pub use arcsinh::SubCommand as MathArcSinH;
pub use arctan::SubCommand as MathArcTan;
pub use arctanh::SubCommand as MathArcTanH;
