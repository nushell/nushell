use crate::matrix::MatrixMN;



/// m = the number of lines (rows) = the height of the matrix
/// n = the number of columns      = the lengths of the matrix
impl MatrixMN {
    pub fn validate_new_matrix_creation(m: usize, n: usize) {
        if m == 0 && n == 0 {
            eprintln!("Err: the new matrix must contain at least one line and one column");
            panic!("Cannot create a matrix with 0 lines nad 0 columns!");
        } else if m == 0 {
            eprintln!("Err: the new matrix must contain at least one line.");
            panic!("Cannot create a matrix with 0 lines!");
        } else if n == 0 {
            eprintln!("Err: the new matrix must contain at least one column.");
            panic!("Cannot create a matrix with 0 columns!");
        }
    }
}

impl MatrixMN {
    /// the function is given a vector that will be transformed into a matrix
    /// as the vector's elements are iterated,
    /// we fill the matrix from left to right, from the upper line to the bottom
    /// vector  : the values of the matrix
    /// m       : the number of lines
    /// n       : the number of columns
    pub fn create_matrix(vector: &Vec<f64>, m: usize, n: usize) -> Self {
        Self::validate_new_matrix_creation(m, n);

        if vector.is_empty() {
            panic!("Cannot a matrix with no elements!");
        }

        if vector.len() != m * n {
            panic!("Invalid size! The matrix expects exactly {} elements", m * n);
        }

        let mut vals = Vec::with_capacity(m);

        for i in 0..=(m - 1) {
            let mut line = Vec::with_capacity(n);

            for j in 0..=(n - 1) {
                let idx: usize = (i * n) + j;
                line.push(vector[idx]);
            }

            vals.push(line);
        }

        return MatrixMN {
            values: vals,
        }
    }
}



impl MatrixMN {
    /// the function will transform a matrix into a vector
    /// using the following algorithm:
    /// the matrix will be traversed from the last line to the last (from upper to bottom)
    /// and each line, from the first element (farthest left) to the last element (farthest right)
    pub fn get_vector(&self) -> Vec<f64> {
        let mut vector: Vec<f64> = Vec::new();

        for line in &self.values {
            for el in line {
                vector.push(*el);
            }
        }

        return vector;
    }
}
