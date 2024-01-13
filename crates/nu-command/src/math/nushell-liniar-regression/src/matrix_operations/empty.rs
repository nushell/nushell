use crate::matrix::MatrixMN;


impl MatrixMN {
    /// creates a matrix with no elements
    pub fn empty() -> Self {
        let vals: Vec<Vec<f64>> = Vec::new();
        return MatrixMN {
            values: vals,
        }
    }
}
