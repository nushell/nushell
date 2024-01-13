use crate::matrix::MatrixMN;


impl MatrixMN {
    /// the function will create the transposed matrix of the input
    /// in the transposed matrix:
    /// the initial columns become rows of the transposed matrix
    /// the initial rows become columns of the transposed matrix
    pub fn transpose(&self) -> Self {
        let m: usize = self.nr_lines();
        let n: usize = self.nr_columns();

        let mut vector: Vec<f64> = Vec::new();

        for j in 0..=(n - 1) {
            for i in 0..=(m - 1) {
                vector.push(self.values[i][j].clone());
            }
        }

        return Self::create_matrix(&vector, n, m);
    }
}
