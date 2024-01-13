use crate::matrix::MatrixMN;

impl MatrixMN {
    pub fn delete_line_column(&self, lin: usize, col: usize) -> Self {

        let mut new_vector: Vec<f64> = Vec::new();

        let m: usize = self.nr_lines();
        let n: usize = self.nr_columns();

        for i in 0..=(m - 1) {
            for j in 0..=(n - 1) {
                if i != lin && j != col {
                    new_vector.push(self.values[i][j]);
                }
            }
        }

        return Self::create_matrix(&new_vector, m - 1, n - 1);
    }
}


impl MatrixMN {
    /// the function calculates the value of a matrix's determinant
    ///
    /// only square matrices have such operation
    pub fn det(&self) -> f64 {
        let mat = MatrixMN::create_matrix(&(self.get_vector()), self.nr_lines(), self.nr_columns());
        return Self::det_helper(mat);
    }

    pub fn det_helper(mat: Self) -> f64 {
        let m: usize = mat.nr_lines();
        let n: usize = mat.nr_columns();

        if m * n == 0 {
            panic!("Empty matrix.");
        } else if m != n {
            panic!("The determinant can be applied only to square matrices.");
        }

        if m == 1 {
            return mat.values[0][0];
        }

        let mut val: f64 = 0.0f64;

        for i in 0..=(m - 1) {
            let cut_mat: Self = mat.delete_line_column(0, i);

            match i % 2 == 0 {
                true => val += mat.values[0][i] * Self::det_helper(cut_mat),
                false => val -= mat.values[0][i] * Self::det_helper(cut_mat),
            }
        }

        return val;
    }
}
