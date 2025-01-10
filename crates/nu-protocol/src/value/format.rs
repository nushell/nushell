use std::fmt::Display;

/// A f64 wrapper that formats whole numbers with a decimal point.
pub struct ObviousFloat(pub f64);

impl Display for ObviousFloat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let val = self.0;
        // This serialises these as 'nan', 'inf' and '-inf', respectively.
        if val.round() == val && val.is_finite() {
            write!(f, "{}.0", val)
        } else {
            write!(f, "{}", val)
        }
    }
}
