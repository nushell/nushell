use crate::matrix::MatrixMN;


impl MatrixMN {
    pub fn unsquare_eye(m: usize, n: usize) -> Self {
        Self::validate_new_matrix_creation(m, n);

        let mut mat: Self = Self::zeros(m, n);

        let idx: usize = if m > n { n } else { m };

        for i in 0..=(idx - 1) {
            mat.values[i][i] = 1.0f64;
        }

        return mat;
    }
}

impl MatrixMN {
    pub fn eye(m: usize) -> Self {
        return Self::unsquare_eye(m, m);
    }
}

