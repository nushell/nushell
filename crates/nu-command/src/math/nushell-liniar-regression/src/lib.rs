
pub mod matrix_operations {
    pub mod create;
    pub mod determinant;
    pub mod empty;
    pub mod eye;
    pub mod inverse;
    pub mod multiply;
    pub mod ones;
    pub mod shape;
    pub mod transpose;
    pub mod zeros;
}

pub mod matrix;
mod xy_data;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}



#[cfg(test)]
mod tests {
    use std::arch::x86_64::{_mm256_stream_pd, _mm_aeskeygenassist_si128};
    use super::*;
    use crate::matrix::MatrixMN;
    use crate::xy_data::*;
    use approx::abs_diff_eq;        // since I compute integrals,
    // I am interested in comparing only the first 5 decimals


    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }


    #[test]
    fn creating_a_matrix() {
        let mut vector: Vec<f64> = Vec::new();

        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let _mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        assert!(true);
    }

    #[test]
    fn check_matrix_dimensions() {
        let mut vector: Vec<f64> = Vec::new();

        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        if mat.nr_lines() != m || mat.height() != m {
            assert!(false);
        }
        if mat.nr_columns() != n || mat.length() != n {
            assert!(false);
        }
        assert!(true);
    }


    #[test]
    fn check_matrix_elements() {
        let mut vector: Vec<f64> = Vec::new();

        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns
        let mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        let values: Vec<f64> = mat.get_vector();

        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }



    #[test]
    /// the function will verify
    /// the elements, the height and the length of the reshaped matrix
    fn change_matrix_dimensions_1() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);
        let values: Vec<f64> = mat.get_vector();

        mat.set_sizes(1, 20);

        if mat.nr_lines() != 1 || mat.height() != 1 {
            assert!(false);
        }
        if mat.nr_columns() != 20 || mat.length() != 20 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    /// the function will verify
    /// the elements, the height and the length of the reshaped matrix
    fn change_matrix_dimensions_2() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector, m, n);
        let values: Vec<f64> = mat.get_vector();

        mat.set_sizes(2, 10);

        if mat.nr_lines() != 2 || mat.height() != 2 {
            assert!(false);
        }
        if mat.nr_columns() != 10 || mat.length() != 10 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }

    #[test]
    /// the function will verify
    /// the elements, the height and the length of the reshaped matrix
    fn change_matrix_dimensions_3() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);
        let values: Vec<f64> = mat.get_vector();

        mat.set_sizes(4, 5);

        if mat.nr_lines() != 4 || mat.height() != 4 {
            assert!(false);
        }
        if mat.nr_columns() != 5 || mat.length() != 5 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }

    #[test]
    /// the function will verify
    /// the elements, the height and the length of the reshaped matrix
    fn change_matrix_dimensions_4() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);
        let values: Vec<f64> = mat.get_vector();

        mat.set_sizes(20, 1);

        if mat.nr_lines() != 20 || mat.height() != 20 {
            assert!(false);
        }
        if mat.nr_columns() != 1 || mat.length() != 1 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }

    #[test]
    #[should_panic]
    fn change_matrix_invalid_dimensions_1() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        // setting a smaller number of elements
        mat.set_sizes(4, 4);        // this should panic!
        assert!(false, "The reshaped matrix must have the same number of elements as the initial matrix does.");
    }

    #[test]
    #[should_panic]
    fn change_matrix_invalid_dimensions_2() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        // setting with a bigger number of elements
        mat.set_sizes(4, 10);       // this should panic!
        assert!(false, "The reshaped matrix must have the same number of elements as the initial matrix does.");
    }


    #[test]
    /// m = the height of the matrix = number of lines of the matrix
    fn reshape_by_setting_height_1() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        mat.set_height(1);      // height = number of lines
        let values: Vec<f64> = mat.get_vector();

        if mat.nr_lines() != 1 || mat.height() != 1 {
            assert!(false);
        }
        if mat.nr_columns() != 20 || mat.length() != 20 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    /// m = the height of the matrix = number of lines of the matrix
    fn reshape_by_setting_height_2() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        mat.set_height(2);      // height = number of lines
        let values: Vec<f64> = mat.get_vector();

        if mat.nr_lines() != 2 || mat.height() != 2 {
            assert!(false);
        }
        if mat.nr_columns() != 10 || mat.length() != 10 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    /// m = the height of the matrix = number of lines of the matrix
    fn reshape_by_setting_height_3() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        mat.set_height(4);      // height = number of lines
        let values: Vec<f64> = mat.get_vector();

        if mat.nr_lines() != 4 || mat.height() != 4 {
            assert!(false);
        }
        if mat.nr_columns() != 5 || mat.length() != 5 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    /// m = the height of the matrix = number of lines of the matrix
    fn reshape_by_setting_height_4() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        mat.set_height(5);      // height = number of lines
        let values: Vec<f64> = mat.get_vector();

        if mat.nr_lines() != 5 || mat.height() != 5 {
            assert!(false);
        }
        if mat.nr_columns() != 4 || mat.length() != 4 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    #[should_panic]
    /// m = the height of the matrix = number of lines of the matrix
    fn invalid_reshape_by_setting_height_1() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector, m, n);

        mat.set_height(0);      // should panic
        assert!(false, "Cannot assign ZERO to be number of lines.");
    }

    #[test]
    #[should_panic]
    /// m = the height of the matrix = number of lines of the matrix
    fn invalid_reshape_by_setting_height_2() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector, m, n);

        mat.set_height(3);      // should panic
        assert!(false, "The number of line must be divisor of the number of all elements.");
    }

    #[test]
    #[should_panic]
    /// m = the height of the matrix = number of lines of the matrix
    fn invalid_reshape_by_setting_height_3() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector, m, n);

        mat.set_height(21);      // should panic
        assert!(false, "The number of line must be divisor of the number of all elements.");
    }


    #[test]
    /// n = the length of the matrix = number of columns of the matrix
    fn reshape_by_setting_length_1() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        mat.set_length(1);      // length = number of columns

        let values: Vec<f64> = mat.get_vector();

        if mat.nr_lines() != 20 || mat.height() != 20 {
            assert!(false);
        }
        if mat.nr_columns() != 1 || mat.length() != 1 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    /// n = the length of the matrix = number of columns of the matrix
    fn reshape_by_setting_length_2() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        mat.set_length(2);      // length = number of columns

        let values: Vec<f64> = mat.get_vector();

        if mat.nr_lines() != 10 || mat.height() != 10 {
            assert!(false);
        }
        if mat.nr_columns() != 2 || mat.length() != 2 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    /// n = the length of the matrix = number of columns of the matrix
    fn reshape_by_setting_length_3() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        mat.set_length(4);      // length = number of columns

        let values: Vec<f64> = mat.get_vector();

        if mat.nr_lines() != 5 || mat.height() != 5 {
            assert!(false);
        }
        if mat.nr_columns() != 4 || mat.length() != 4 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    /// n = the length of the matrix = number of columns of the matrix
    fn reshape_by_setting_length_4() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector,m, n);

        mat.set_length(5);      // length = number of columns
        let values: Vec<f64> = mat.get_vector();

        if mat.nr_lines() != 4 || mat.height() != 4 {
            assert!(false);
        }
        if mat.nr_columns() != 5 || mat.length() != 5 {
            assert!(false);
        }
        if values.len() != vector.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values[i] != vector[i] {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    #[should_panic]
    /// n = the length of the matrix = number of columns of the matrix
    fn invalid_reshape_by_setting_length_1() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector, m, n);

        mat.set_length(0);      // should panic
        assert!(false, "Cannot assign ZERO to be number of lines.");
    }

    #[test]
    #[should_panic]
    /// n = the length of the matrix = number of columns of the matrix
    fn invalid_reshape_by_setting_length_2() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector, m, n);

        mat.set_length(3);      // should panic
        assert!(false, "The number of columns must be divisor of the number of all elements.");
    }

    #[test]
    #[should_panic]
    /// n = the length of the matrix = number of columns of the matrix
    fn invalid_reshape_by_setting_length_3() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mut mat: MatrixMN = MatrixMN::create_matrix(&vector, m, n);

        mat.set_length(21);      // should panic
        assert!(false, "The number of columns must be divisor of the number of all elements.");
    }

    #[test]
    fn resize_matrix_1() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mat1: MatrixMN = MatrixMN::create_matrix(&vector, m, n);
        let values1: Vec<f64> = mat1.get_vector();

        let mat2: MatrixMN = mat1.resize(1, 20);
        let values2: Vec<f64> = mat2.get_vector();

        if mat2.nr_lines() != 1 || mat2.height() != 1 {
            assert!(false);
        }
        if mat2.nr_columns() != 20 || mat2.length() != 20 {
            assert!(false);
        }

        if values1.len() != values2.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values1[i] != values2[i] {
                assert!(false);
            }

            assert!(true);
        }
    }

    #[test]
    fn resize_matrix_2() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mat1: MatrixMN = MatrixMN::create_matrix(&vector, m, n);
        let values1: Vec<f64> = mat1.get_vector();

        let mat2: MatrixMN = mat1.resize(2, 10);
        let values2: Vec<f64> = mat2.get_vector();

        if mat2.nr_lines() != 2 || mat2.height() != 2 {
            assert!(false);
        }
        if mat2.nr_columns() != 10 || mat2.length() != 10 {
            assert!(false);
        }

        if values1.len() != values2.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values1[i] != values2[i] {
                assert!(false);
            }

            assert!(true);
        }
    }

    #[test]
    fn resize_matrix_3() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mat1: MatrixMN = MatrixMN::create_matrix(&vector, m, n);
        let values1: Vec<f64> = mat1.get_vector();

        let mat2: MatrixMN = mat1.resize(4, 5);
        let values2: Vec<f64> = mat2.get_vector();

        if mat2.nr_lines() != 4 || mat2.height() != 4 {
            assert!(false);
        }
        if mat2.nr_columns() != 5 || mat2.length() != 5 {
            assert!(false);
        }

        if values1.len() != values2.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values1[i] != values2[i] {
                assert!(false);
            }

            assert!(true);
        }
    }

    #[test]
    fn resize_matrix_4() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mat1: MatrixMN = MatrixMN::create_matrix(&vector, m, n);
        let values1: Vec<f64> = mat1.get_vector();

        let mat2: MatrixMN = mat1.resize(5, 4);
        let values2: Vec<f64> = mat2.get_vector();

        if mat2.nr_lines() != 5 || mat2.height() != 5 {
            assert!(false);
        }
        if mat2.nr_columns() != 4 || mat2.length() != 4 {
            assert!(false);
        }

        if values1.len() != values2.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values1[i] != values2[i] {
                assert!(false);
            }

            assert!(true);
        }
    }

    #[test]
    fn resize_matrix_5() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mat1: MatrixMN = MatrixMN::create_matrix(&vector, m, n);
        let values1: Vec<f64> = mat1.get_vector();

        let mat2: MatrixMN = mat1.resize(10, 2);
        let values2: Vec<f64> = mat2.get_vector();

        if mat2.nr_lines() != 10 || mat2.height() != 10 {
            assert!(false);
        }
        if mat2.nr_columns() != 2 || mat2.length() != 2 {
            assert!(false);
        }

        if values1.len() != values2.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values1[i] != values2[i] {
                assert!(false);
            }

            assert!(true);
        }
    }

    #[test]
    fn resize_matrix_6() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mat1: MatrixMN = MatrixMN::create_matrix(&vector, m, n);
        let values1: Vec<f64> = mat1.get_vector();

        let mat2: MatrixMN = mat1.resize(20, 1);
        let values2: Vec<f64> = mat2.get_vector();

        if mat2.nr_lines() != 20 || mat2.height() != 20 {
            assert!(false);
        }
        if mat2.nr_columns() != 1 || mat2.length() != 1 {
            assert!(false);
        }

        if values1.len() != values2.len() {
            assert!(false);
        }

        for i in 0..=19 {
            if values1[i] != values2[i] {
                assert!(false);
            }

            assert!(true);
        }
    }

    #[test]
    #[should_panic]
    fn invalid_resize_matrix_1() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mat1: MatrixMN = MatrixMN::create_matrix(&vector, m, n);

        // setting a smaller number of elements
        let _mat2: MatrixMN = mat1.resize(4, 4);     // this should panic!
        assert!(false, "The new resized matrix must have the same number of elements as the initial one.");
    }

    #[test]
    #[should_panic]
    fn invalid_resize_matrix_2() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=20 {
            vector.push(i as f64);
        }

        let m: usize = 2;       // number of lines
        let n: usize = 10;      // number of columns

        let mat1: MatrixMN = MatrixMN::create_matrix(&vector, m, n);

        // setting a bigger number of elements
        let _mat2: MatrixMN = mat1.resize(5, 5);     // this should panic!
        assert!(false, "The new resized matrix must have the same number of elements as the initial one.");
    }


    #[test]
    fn create_identical_square_matrices() {
        for i in 1..=15 {
            // identical square matrix with `i` lines and `i` columns
            // eye(3) should look like this
            // 1 0 0
            // 0 1 0
            // 0 0 1
            let identical: MatrixMN = MatrixMN::eye(i);
            if identical.height() != i || identical.nr_lines() != i {
                assert!(false);
            }
            if identical.length() != i || identical.nr_columns() != i {
                assert!(false);
            }
            for j in 0..=(i - 1) {
                for k in 0..=(i - 1) {
                    if j == k && identical.values[j][k] != 1.0 {
                        assert!(false);
                    }
                    if j != k && identical.values[j][k] != 0.0 {
                        assert!(false);
                    }
                }
            }

            assert!(true);
        }
    }


    #[test]
    fn create_matrices_with_zeros() {
        // matrix with 10 lines and 2 columns
        let mut mat: MatrixMN = MatrixMN::zeros(10, 2);
        if mat.height() != 10 || mat.nr_lines() != 10 {
            assert!(false);
        }
        if mat.length() != 2 || mat.nr_columns() != 2 {
            assert!(false);
        }
        for i in 0..=9 {
            for j in 0..=1 {
                if mat.values[i][j] != 0.0 {
                    assert!(false);
                }
            }
        }

        // matrix with 5 lines and 7 columns
        mat = MatrixMN::zeros(5, 7);
        if mat.height() != 5 || mat.nr_lines() != 5 {
            assert!(false);
        }
        if mat.length() != 7 || mat.nr_columns() != 7 {
            assert!(false);
        }
        for i in 0..=4 {
            for j in 0..=6 {
                if mat.values[i][j] != 0.0 {
                    assert!(false);
                }
            }
        }

        assert!(true);
    }

    #[test]
    fn transpose_matrix_1() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0 , 4.0], 1, 4);
        // 1.0 2.0 3.0 4.0


        let mat_t: MatrixMN = mat.transpose();
        mat_t.disp();
        // 1.0
        // 2.0
        // 3.0
        // 4.0

        if mat_t.nr_lines() != 4 || mat_t.height() != 4 {
            assert!(false);
        }
        if mat_t.nr_columns() != 1 || mat_t.length() != 1 {
            assert!(false);
        }

        for i in 0..=0 {
            for j in 0..=3 {
                if mat.values[i][j] != mat_t.values[j][i] {
                    assert!(false);
                }
            }
        }

        assert!(true);
    }

    #[test]
    fn transpose_matrix_2() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0 , 4.0], 4, 1);
        // 1.0
        // 2.0
        // 3.0
        // 4.0

        let mat_t: MatrixMN = mat.transpose();
        // 1.0 2.0 3.0 4.0


        if mat_t.nr_lines() != 1 || mat_t.height() != 1 {
            assert!(false);
        }
        if mat_t.nr_columns() != 4 || mat_t.length() != 4 {
            assert!(false);
        }

        for i in 0..=3 {
            for j in 0..=0 {
                if mat.values[i][j] != mat_t.values[j][i] {
                    assert!(false);
                }
            }
        }

        assert!(true);
    }
    #[test]
    fn transpose_matrix_3() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0 , 4.0, 5.0, 6.0, 7.0, 8.0], 2, 4);
        // 1.0 2.0 3.0 4.0
        // 5.0 6.0 7.0 8.0


        let mat_t: MatrixMN = mat.transpose();
        // 1.0 5.0
        // 2.0 6.0
        // 3.0 7.0
        // 4.0 8.0


        if mat_t.nr_lines() != 4 || mat_t.height() != 4 {
            assert!(false);
        }
        if mat_t.nr_columns() != 2 || mat_t.length() != 2 {
            assert!(false);
        }

        for i in 0..=1 {
            for j in 0..=3 {
                if mat.values[i][j] != mat_t.values[j][i] {
                    assert!(false);
                }
            }
        }

        assert!(true);
    }

    #[test]
    fn transposed_transposed_matrix() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0 , 4.0, 5.0, 6.0, 7.0, 8.0], 2, 4);
        // 1.0 2.0 3.0 4.0
        // 5.0 6.0 7.0 8.0


        let mat_t: MatrixMN = mat.transpose();
        // 1.0 5.0
        // 2.0 6.0
        // 3.0 7.0
        // 4.0 8.0

        let mat_tt: MatrixMN = mat_t.transpose();
        // 1.0 2.0 3.0 4.0
        // 5.0 6.0 7.0 8.0

        if mat_tt.nr_lines() != 2 || mat_tt.height() != 2 {
            assert!(false);
        }
        if mat_tt.nr_columns() != 4 || mat_tt.length() != 4 {
            assert!(false);
        }

        for i in 0..=1 {
            for j in 0..=3 {
                if mat.values[i][j] != mat_tt.values[i][j] {
                    assert!(false);
                }
            }
        }

        assert!(true);
    }

    #[test]
    fn multiply_vector_matrices_1() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0], 1, 3);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![3.0, 2.0, 1.0], 3, 1);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);

        if mat3.length() != 1 || mat3.nr_lines() != 1 {
            assert!(false);
        }
        if mat3.height() != 1 || mat3.nr_columns() != 1 {
            assert!(false);
        }

        if mat3.values[0][0] != 10.0 {
            assert!(false);
        }

        assert!(true);
    }


    #[test]
    #[should_panic]
    fn invalid_multiply_vector_matrices_1() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0], 1, 3);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![3.0, 2.0, 1.0], 3, 1);
        let _mat3: MatrixMN = MatrixMN::mul(&mat2, &mat1);     // should panic!
        assert!(false, "The function should panic! \
        The operation can be applied only to matrices for which \
        the number of columns of the first one equals \
        the number of rows of the second one.");
    }

    #[test]
    fn multiply_vector_matrices_2() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![3.0, 2.0, 1.0], 3, 1);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0], 1, 3);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);

        if mat3.length() != 3 || mat3.nr_lines() != 3 {
            assert!(false);
        }
        if mat3.height() != 3 || mat3.nr_columns() != 3 {
            assert!(false);
        }

        if mat3.values[0][0] != 3.0 || mat3.values[0][1] != 6.0 || mat3.values[0][2] != 9.0
            || mat3.values[1][0] != 2.0 || mat3.values[1][1] != 4.0 || mat3.values[1][2] != 6.0
            || mat3.values[2][0] != 1.0 || mat3.values[2][1] != 2.0 || mat3.values[2][2] != 3.0 {
            assert!(false)
        }
    }

    #[test]
    #[should_panic]
    fn invalid_multiply_vector_matrices_2() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0], 3, 1);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![3.0, 2.0, 1.0], 1, 3);
        let _mat3: MatrixMN = MatrixMN::mul(&mat2, &mat1);
        assert!(false, "The function should panic! \
        The operation can be applied only to matrices for which \
        the number of columns of the first one equals \
        the number of rows of the second one.");
    }

    #[test]
    fn multiply_square_matrices_1() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![4.0, 1.0, 3.0, 2.0], 2, 2);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);

        if mat3.length() != 2 || mat3.nr_lines() != 2 {
            assert!(false);
        }
        if mat3.height() != 2 || mat3.nr_columns() != 2 {
            assert!(false);
        }

        if mat3.values[0][0] != 10.0 || mat3.values[0][1] != 5.0
            || mat3.values[1][0] != 24.0 || mat3.values[1][1] != 11.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn multiply_square_matrices_2() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![4.0, 1.0, 3.0, 2.0], 2, 2);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);

        if mat3.length() != 2 || mat3.nr_lines() != 2 {
            assert!(false);
        }
        if mat3.height() != 2 || mat3.nr_columns() != 2 {
            assert!(false);
        }

        if mat3.values[0][0] != 7.0 || mat3.values[0][1] != 12.0
            || mat3.values[1][0] != 9.0 || mat3.values[1][1] != 14.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn multiply_non_square_matrices_1() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0], 3, 2);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);

        if mat3.values[0][0] != 14.0 || mat3.values[0][1] != 32.0
            || mat3.values[1][0] != 32.0 || mat3.values[1][1] != 77.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn multiply_non_square_matrices_2() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 2, 4);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 5.0, 2.0, 6.0, 3.0, 7.0, 4.0, 8.0], 4, 2);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);

        if mat3.values[0][0] != 30.0 || mat3.values[0][1] != 70.0
            || mat3.values[1][0] != 70.0 || mat3.values[1][1] != 174.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn multiply_non_square_matrices_3() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 5.0, 2.0, 6.0, 3.0, 7.0, 4.0, 8.0], 4, 2);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 2, 4);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);

        if mat3.values[0][0] != 26.0 || mat3.values[0][1] != 32.0
            || mat3.values[0][2] != 38.0 || mat3.values[0][3] != 44.0
            || mat3.values[1][0] != 32.0 || mat3.values[1][1] != 40.0
            || mat3.values[1][2] != 48.0 || mat3.values[1][3] != 56.0
            || mat3.values[2][0] != 38.0 || mat3.values[2][1] != 48.0
            || mat3.values[2][2] != 58.0 || mat3.values[2][3] != 68.0
            || mat3.values[3][0] != 44.0 || mat3.values[3][1] != 56.0
            || mat3.values[3][2] != 68.0 || mat3.values[3][3] != 80.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn multiply_non_square_matrices_4() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 5.0], 1, 2);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 2, 4);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);

        if mat3.values[0][0] != 26.0 || mat3.values[0][1] != 32.0
            || mat3.values[0][2] != 38.0 || mat3.values[0][3] != 44.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn multiply_non_square_matrices_5() {
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 5.0], 1, 2);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 5.0, 6.0, 7.0], 2, 3);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);

        if mat3.values[0][0] != 26.0 || mat3.values[0][1] != 32.0 || mat3.values[0][2] != 38.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn determinant_eyes() {
        for i in 1..=10 {
            let mat: MatrixMN = MatrixMN::eye(i);
            if mat.det() != 1.0 {
                assert!(false);
            }
        }
        assert!(true);
    }

    #[test]
    fn determinant_zeros() {
        for i in 1..=10 {
            let mat: MatrixMN = MatrixMN::zeros(i, i);
            if mat.det() != 0.0 {
                assert!(false);
            }
        }
        assert!(true);
    }

    #[test]
    fn determinant_1() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![1.0], 1, 1);
        assert_eq!(mat.det(), 1.0);
    }

    #[test]
    fn determinant_2() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        assert_eq!(mat.det(), -2.0);
    }

    #[test]
    fn determinant_3() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=9 {
            vector.push(i as f64);
        }

        let mat: MatrixMN = MatrixMN::create_matrix(&vector, 3, 3);
        assert_eq!(mat.det(), 0.0);
    }

    #[test]
    fn determinant_4() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=16 {
            vector.push(i as f64);
        }

        let mat: MatrixMN = MatrixMN::create_matrix(&vector, 4, 4);
        assert_eq!(mat.det(), 0.0);
    }

    #[test]
    fn determinant_5() {
        let vals: Vec<f64> = vec![1.0, 1.0, 2.0, 4.0, 5.0, 7.0, 8.0, 9.0, 10.0];
        let mat: MatrixMN = MatrixMN::create_matrix(&vals, 3, 3);
        assert_eq!(mat.det(), -5.0);
    }

    #[test]
    fn determinant_6() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=25 {
            vector.push(i as f64);
        }

        let mat: MatrixMN = MatrixMN::create_matrix(&vector, 5, 5);
        assert_eq!(mat.det(), 0.0);
    }

    #[test]
    #[should_panic]
    fn invalid_determinant_1() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=16 {
            vector.push(i as f64);
        }

        let mat: MatrixMN = MatrixMN::create_matrix(&vector, 2, 8);
        let det: f64 = mat.det();       // should panic
        assert!(false, "Should panic! The determinant can be applied only to square matrices.");
    }

    #[test]
    #[should_panic]
    fn invalid_determinant_2() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=16 {
            vector.push(i as f64);
        }

        let mat: MatrixMN = MatrixMN::create_matrix(&vector, 16, 1);
        let det: f64 = mat.det();       // should panic
        assert!(false, "Should panic! The determinant can be applied only to square matrices.");
    }


    #[test]
    fn inverse_matrix_1() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![1.0], 1, 1);
        if !mat.is_invertible() {
            assert!(false);
        }
        let inv: MatrixMN = mat.inverse();

        if inv.height() != 1 || inv.nr_lines() != 1 {
            assert!(false);
        }
        if inv.length() != 1 || inv.nr_columns() != 1 {
            assert!(false);
        }
        if inv.values[0][0] != 1.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn inverse_matrix_2() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![5.0], 1, 1);
        if !mat.is_invertible() {
            assert!(false);
        }
        let inv: MatrixMN = mat.inverse();

        if inv.height() != 1 || inv.nr_lines() != 1 {
            assert!(false);
        }
        if inv.length() != 1 || inv.nr_columns() != 1 {
            assert!(false);
        }
        if inv.values[0][0] != 0.2 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn inverse_matrix_3() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![-0.5], 1, 1);
        if !mat.is_invertible() {
            assert!(false);
        }
        let inv: MatrixMN = mat.inverse();

        if inv.height() != 1 || inv.nr_lines() != 1 {
            assert!(false);
        }
        if inv.length() != 1 || inv.nr_columns() != 1 {
            assert!(false);
        }
        inv.disp();
        if inv.values[0][0] != -2.0 {
            assert!(false);
        }

        assert!(true);
    }
    #[test]
    fn inverse_matrix_4() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        if !mat.is_invertible() {
            assert!(false);
        }
        let inv: MatrixMN = mat.inverse();

        if inv.height() != 2 || inv.nr_lines() != 2 {
            assert!(false);
        }
        if inv.length() != 2 || inv.nr_columns() != 2 {
            assert!(false);
        }
        inv.disp();
        if inv.values[0][0] != -2.0 || inv.values[0][1] != 1.0
            || inv.values[1][0] != 1.5 || inv.values[1][1] != -0.5 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn inverse_matrix_5() {
        let vals: Vec<f64> = vec![1.0, 1.0, 2.0, 4.0, 5.0, 7.0, 8.0, 9.0, 10.0];
        let mat: MatrixMN = MatrixMN::create_matrix(&vals, 3, 3);
        if !mat.is_invertible() {
            assert!(false);
        }
        let inv: MatrixMN = mat.inverse();

        if inv.height() != 3 || inv.nr_lines() != 3 {
            assert!(false);
        }
        if inv.length() != 3 || inv.nr_columns() != 3 {
            assert!(false);
        }
        inv.disp();
        if inv.values[0][0] != 13.0/5.0  || inv.values[0][1] != -8.0/5.0 || inv.values[0][2] != 3.0/5.0
            || inv.values[1][0] != -16.0/5.0 || inv.values[1][1] != 6.0/5.0 || inv.values[1][2] != -1.0/5.0
            || inv.values[2][0] != 4.0/5.0 || inv.values[2][1] != 1.0/5.0 || inv.values[2][2] != -1.0/5.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn inverse_matrix_6() {
        let vals: Vec<f64> = vec![1.0, 1.0, 2.0, 4.0, 5.0, 7.0, 8.0, 9.0, 10.0];
        let mat: MatrixMN = MatrixMN::create_matrix(&vals, 3, 3);
        let matt: MatrixMN = mat.transpose();

        if !matt.is_invertible() {
            assert!(false);
        }

        let inv: MatrixMN = matt.inverse();
        let invt: MatrixMN = inv.transpose();

        if inv.height() != 3 || inv.nr_lines() != 3 {
            assert!(false);
        }
        if inv.length() != 3 || inv.nr_columns() != 3 {
            assert!(false);
        }

        if invt.values[0][0] != 13.0/5.0  || invt.values[0][1] != -8.0/5.0 || invt.values[0][2] != 3.0/5.0
            || invt.values[1][0] != -16.0/5.0 || invt.values[1][1] != 6.0/5.0 || invt.values[1][2] != -1.0/5.0
            || invt.values[2][0] != 4.0/5.0 || invt.values[2][1] != 1.0/5.0 || invt.values[2][2] != -1.0/5.0 {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn inverse_invalid_1() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![0.0], 1, 1);
        assert_eq!(mat.is_invertible(), false);
    }

    #[test]
    fn inverse_invalid_2() {
        let mat: MatrixMN = MatrixMN::create_matrix(&vec![2.0, -1.0, -8.0, 4.0], 2, 2);
        assert_eq!(mat.is_invertible(), false);
    }

    #[test]
    fn inverse_invalid_3() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=9 {
            vector.push(i as f64);
        }

        let mat: MatrixMN = MatrixMN::create_matrix(&vector, 3, 3);
        assert_eq!(mat.is_invertible(), false);
    }

    #[test]
    fn inverse_invalid_4() {
        let mut vector: Vec<f64> = Vec::new();
        for i in 1..=16 {
            vector.push(i as f64);
        }

        let mat: MatrixMN = MatrixMN::create_matrix(&vector, 4, 4);
        assert_eq!(mat.is_invertible(), false);
    }


    #[test]
    fn multiply_two_matrices_1() {
        assert!(true);
    }

    #[test]
    fn test_all_matrix_functions() {
        println!("\nCreating a matrix:");
        let vals: Vec<f64> = vec![1.0, 1.0, 2.0, 4.0, 5.0, 7.0, 8.0, 9.0, 10.0];
        let mut mat: MatrixMN = MatrixMN::create_matrix(&vals, 3, 3);
        mat.disp();         // prints the matrix to stdout

        println!("\nCreating an identical 4x4 matrix:");
        mat = MatrixMN::eye(4);
        mat.disp();

        println!("\nCreating an identical matrix with 3 lines and 5 columns:");
        mat = MatrixMN::unsquare_eye(3, 5);
        mat.disp();

        println!("\nCreating an identical matrix with 5 lines and 4 columns:");
        mat = MatrixMN::unsquare_eye(5, 3);
        mat.disp();


        println!("\nTransposing a matrix:");
        println!("Initial matrix:");
        mat = MatrixMN::create_matrix(&vals, 3, 3);
        println!("Transposed matrix:");
        let matt: MatrixMN = mat.transpose();
        matt.disp();

        println!("\nMultiply two matrices");
        let mat1: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 2, 4);
        let mat2: MatrixMN = MatrixMN::create_matrix(&vec![1.0, 5.0, 2.0, 6.0, 3.0, 7.0, 4.0, 8.0], 4, 2);
        let mat3: MatrixMN = MatrixMN::mul(&mat1, &mat2);
        mat3.disp();

        println!("\nThe inverse of a matrix");
        let inv: MatrixMN = mat.inverse();
        inv.disp();

        assert!(true);
    }

    #[test]
    fn create_data_set_1() {
        let dt: DataSet = DataSet {
            x_name: String::from("X"),
            y_name: String::from("Y"),
            x_values: vec![1.0, 2.0, 3.0, 4.0],
            y_values: vec![11.0, 15.0, 26.0, 55.0],
        };

        dt.validate_data_set();
        assert!(true);
    }

    #[test]
    fn create_data_set_2() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let val2: Vec<f64> = vec![11.0, 15.0, 26.0, 55.0];

        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        if dt.x_name != "X-var" || dt.y_name != "Y-var" {
            assert!(false);
        }

        if dt.x_values.len() != dt.y_values.len()
            || dt.x_values.len() != val1.len() || dt.y_values.len() != val2.len() {
            assert!(false);
        }

        for i in 0..dt.x_values.len() {
            if dt.x_values[i] != val1[i] || dt.y_values[i] != val2[i] {
                assert!(false);
            }
        }

        assert!(true);
    }

    #[test]
    #[should_panic]
    fn create_invalid_data_set_1() {
        let dt: DataSet = DataSet {
            x_name: String::from(""),
            y_name: String::from("Y"),
            x_values: vec![1.0, 2.0, 3.0, 4.0],
            y_values: vec![11.0, 15.0, 26.0, 55.0],
        };

        dt.validate_data_set();
        assert!(true);
    }

    #[test]
    #[should_panic]
    fn create_invalid_data_set_2() {
        let dt: DataSet = DataSet {
            x_name: String::from("X"),
            y_name: String::new(),
            x_values: vec![1.0, 2.0, 3.0, 4.0],
            y_values: vec![11.0, 15.0, 26.0, 55.0],
        };

        dt.validate_data_set();
        assert!(true);
    }

    #[test]
    #[should_panic]
    fn create_invalid_data_set_3() {
        let dt: DataSet = DataSet {
            x_name: String::from("X"),
            y_name: String::from("Y"),
            x_values: vec![],
            y_values: vec![11.0, 15.0, 26.0, 55.0],
        };

        dt.validate_data_set();
        assert!(true);
    }

    #[test]
    #[should_panic]
    fn create_invalid_data_set_4() {
        let dt: DataSet = DataSet {
            x_name: String::from("X"),
            y_name: String::from("Y"),
            x_values: vec![1.0, 2.0, 3.0, 4.0],
            y_values: Vec::new(),
        };

        dt.validate_data_set();
        assert!(true);
    }

    #[test]
    #[should_panic]
    fn create_invalid_data_set_5() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 11.5];      // len = 5
        let val2: Vec<f64> = vec![11.0, 15.0, 26.0, 55.0];        // len = 4

        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());
        assert!(true);
    }


    #[test]
    fn create_data_set_from_vector_1() {
        let val2: Vec<f64> = vec![1.0, 2.0];

        let dt: DataSet = DataSet::new_from_vec(val2.clone());

        if dt.x_name != "X" || dt.y_name != "Y" {
            assert!(false);
        }

        // check the X-axis
        if dt.x_values[0] != 0.0 || dt.x_values[1] != 1.0 {
            assert!(false);
        }

        // check the Y-axis
        if dt.y_values[0] != 1.0 || dt.y_values[1] != 2.0 {
            assert!(false);
        }

        assert!(true);
    }


    #[test]
    #[should_panic]
    fn create_invalid_data_set_from_vector_1() {
        let val2: Vec<f64> = vec![];
        let dt: DataSet = DataSet::new_from_vec(val2.clone());
        assert!(true);
    }

    #[test]
    fn linear_regression_1() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let val2: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0];

        // EQUATION d : y = 10 * x
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        // the line will be the first bisector
        match dt.compute_linear_regression() {
            Ok(line) => {
                println!("{}", dt.equation_linear_regression());
                // Compare with a precision of the first 10 decimals

                let precision = 1e-10;

                if abs_diff_eq!(line.slope, 10.0, epsilon = precision) == false {
                    assert!(false);
                }

                if abs_diff_eq!(line.intercept, 0.0, epsilon = precision) == false {
                    assert!(false);
                }

            },
            Err(_) => assert!(false),
        }
        assert!(true);
    }


    #[test]
    fn linear_regression_2() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let val2: Vec<f64> = vec![11.0, 11.0, 11.0, 11.0];

        // EQUATION d : y = 11
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        // the line will be a line parallel to OY
        match dt.compute_linear_regression() {
            Ok(line) => {
                // d : y = a * x + b ; a = slope; b = intercept
                let precision = 1e-10;

                if abs_diff_eq!(line.slope, 0.0, epsilon = precision) == false {
                    assert!(false);
                }

                if abs_diff_eq!(line.intercept, 11.0, epsilon = precision) == false {
                    assert!(false);
                }

                assert!(true);
            },
            Err(_) => {
                assert!(false);
            },
        }
        assert!(true);
    }


    #[test]
    fn linear_regression_3() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![7.0, 7.0, 7.0, 7.0];
        let val2: Vec<f64> = vec![1.0, 1.5, 2.5, 4.0];

        // EQUATION d : x = 7
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        match dt.compute_linear_regression() {
            Ok(_) => {
                assert!(false);
            },
            Err(xbar) => {
                let precision = 1e-10;

                if abs_diff_eq!(xbar.x, 7.0, epsilon = precision) == false {
                    assert!(false);
                }
                assert!(true);
            }
        }

        assert!(true);
    }

    #[test]
    fn linear_regression_4() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let val2: Vec<f64> = vec![14.5, 140.1, 201.3, 220.5];

        // EQUATION d : y = 67.92 * x - 25.70
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        match dt.compute_linear_regression() {
            Ok(line) => {
                // d : y = a * x + b ; a = slope; b = intercept
                let precision = 1e-2;

                if abs_diff_eq!(line.slope, 67.92, epsilon = precision) == false {
                    assert!(false);
                }

                if abs_diff_eq!(line.intercept, -25.70, epsilon = precision) == false {
                    assert!(false);
                }

                assert!(true);
            },
            Err(_) => {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    fn linear_regression_5() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let val2: Vec<f64> = vec![220.5, 201.3, 140.1, 14.5];

        // EQUATION d : y = -67.919 * x + 313.9
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        match dt.compute_linear_regression() {
            Ok(line) => {
                // d : y = a * x + b ; a = slope; b = intercept
                let precision = 1e-2;

                if abs_diff_eq!(line.slope, -67.919, epsilon = precision) == false {
                    assert!(false);
                }

                if abs_diff_eq!(line.intercept, 313.900, epsilon = precision) == false {
                    assert!(false);
                }

                assert!(true);
            },
            Err(_) => {
                assert!(false);
            }
        }

        assert!(true);
    }


    #[test]
    fn verify_equation_1() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let val2: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0];

        // EQUATION d : y = 10 * x
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        // println!("{:?}", dt.equation_linear_regression());
        assert_eq!(dt.equation_linear_regression(), "d : y = 10.0000000000 * x");
    }


    #[test]
    fn verify_equation_2() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let val2: Vec<f64> = vec![11.0, 11.0, 11.0, 11.0];

        // EQUATION d : y = 11
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        // println!("{:?}", dt.equation_linear_regression());
        assert_eq!(dt.equation_linear_regression(), "d : y = 11.0000000000");
    }

    #[test]
    fn verify_equation_3() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![7.0, 7.0, 7.0, 7.0];
        let val2: Vec<f64> = vec![1.0, 1.5, 2.5, 4.0];

        // EQUATION d : x = 7
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        // println!("{:?}", dt.equation_linear_regression());
        assert_eq!(dt.equation_linear_regression(), "d : x = 7.0000000000");
    }

    #[test]
    fn verify_equation_4() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let val2: Vec<f64> = vec![14.5, 140.1, 201.3, 220.5];

        // EQUATION d : y = 67.92 * x - 25.70
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        // println!("{:?}", dt.equation_linear_regression());
        assert_eq!(dt.equation_linear_regression(), "d : y = 67.9200000000 * x - 25.7000000000");
    }

    #[test]
    fn verify_equation_5() {
        let nm1: String = String::from("X-var");
        let nm2: String = String::from("Y-var");
        let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let val2: Vec<f64> = vec![220.5, 201.3, 140.1, 14.5];

        // EQUATION d : y = -67.919 * x + 313.9
        let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

        // to display : println!("{:?}", dt.equation_linear_regression());
        assert_eq!(dt.equation_linear_regression(), "d : y = -67.9200000000 * x + 313.9000000000");
    }
}
