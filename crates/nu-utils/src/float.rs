use std::fmt::{Display, LowerExp};

/// A f64 wrapper that formats whole numbers with a decimal point.
pub struct ObviousFloat(pub f64);

impl Display for ObviousFloat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let val = self.0;
        if val.fract() == 0.0 {
            write!(f, "{val:.1}")
        } else {
            Display::fmt(&val, f)
        }
    }
}

impl LowerExp for ObviousFloat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        LowerExp::fmt(&self.0, f)
    }
}
