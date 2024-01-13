use crate::matrix::MatrixMN;

impl MatrixMN {
    /// the number of columns will be returned
    pub fn length(&self) -> usize {
        return self.nr_columns();
    }

    /// the number of lines will be returned
    pub fn height(&self) -> usize {
        return self.nr_lines();
    }


    /// the number of columns will be returned
    pub fn nr_columns(&self) ->usize {
        if self.values.is_empty() {
            return 0;
        }
        return self.values[0].len();
    }

    /// the number of the lines will be returned
    pub fn nr_lines(&self) -> usize {
        if self.values.is_empty() {
            return 0;
        }
        return self.values.len();
    }
}


impl MatrixMN {
    /// the function is given a matrix
    pub fn resize(&self, m: usize, n: usize) -> Self {
        let vector: Vec<f64> = self.get_vector();

        Self::validate_new_matrix_creation(m, n);

        if vector.is_empty() {
            panic!("Cannot a matrix with no elements!");
        }

        if vector.len() != m * n {
            panic!("Invalid size! The matrix expects exactly {} elements", m * n);
        }

        return Self::create_matrix(&vector, m, n);
    }
}


impl MatrixMN {
    ///
    pub fn set_sizes(&mut self, m:usize, n: usize) {
        let vector: Vec<f64> = self.get_vector();

        if vector.is_empty() {
            panic!("Cannot create an empty matrix!");
        }

        if vector.len() != m * n {
            panic!("Invalid size! The matrix expects exactly {} elements", m * n);
        }


        let new_mat: Self = Self::create_matrix(&vector, m, n);
        self.values = new_mat.values;
    }
}

impl MatrixMN {
    /// length = number of column
    pub fn set_length(&mut self, n: usize) {
        if n == 0 {
            panic!("Cannot set the number of column with 0.");
        }

        let vector: Vec<f64> = self.get_vector();

        if vector.len() % n != 0 {
            eprintln!("Err: cannot split the matrix in {} columns", n);
            eprintln!("Err: The length (number of columns) must be a divisor of {}", vector.len());
            panic!("Invalid resize");
        }

        let new_mat: Self = Self::create_matrix(&vector, vector.len() / n, n);
        self.values = new_mat.values;
    }
}

impl MatrixMN {
    /// height = number of rows (lines)
    pub fn set_height(&mut self, m: usize) {

        if m == 0 {
            panic!("Cannot set the number of lines with 0.");
        }

        let vector: Vec<f64> = self.get_vector();

        if vector.len() % m != 0 {
            eprintln!("Err: cannot split the matrix in {} lines.", m);
            eprintln!("Err: The height (number of lines) must be a divisor of {}", vector.len());
            panic!("Invalid resize");
        }

        let new_mat: Self = Self::create_matrix(&vector, m, vector.len() / m);
        self.values = new_mat.values;
    }
}
