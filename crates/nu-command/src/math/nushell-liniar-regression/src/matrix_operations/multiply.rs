use crate::matrix::MatrixMN;


impl MatrixMN {
    /// the function multiplies two matrices
    ///
    /// for a valid operation
    /// the number of columns of the first matrix
    /// must equal
    /// the number of rows of the second matrix
    pub fn mul(mat1: &Self, mat2: &Self) -> Self {
        let m1: usize = mat1.nr_lines();
        let n1: usize = mat1.nr_columns();

        let m2: usize = mat2.nr_lines();
        let n2: usize = mat2.nr_columns();

        match (m1 * n1 == 0, m2 * n2 == 0) {
            (true, true) => panic!("Both matrices are empty"),
            (true, false) => panic!("The first matrix is empty"),
            (false, true) => panic!("The first matrix is empty"),
            _ => (),

        }

        if n1 != m2 {
            panic!("The number of COLUMNS of the FIRST matrix must equal \
                    the number of LINES of the SECOND matrix");
        }

        let m: usize = m1;      // nr lines first matrix == nr lines final matrix
        let p: usize = m2;      // nr cols first matrix = nr lines second matrix
        let n: usize = n2;      // nr cols second matrix == nr cols final matrix
        let mut ret_mat: MatrixMN = Self::zeros(m, n);

        for i in 0..=(m - 1) {
            for j in 0..=(n - 1) {

                ret_mat.values[i][j] = 0.0f64;
                for k in 0..=(p - 1) {
                    ret_mat.values[i][j] += mat1.values[i][k] * mat2.values[k][j];
                }

            }
        }

        return ret_mat;
    }
}
