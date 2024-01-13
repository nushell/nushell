

pub struct MatrixMN {
    pub values: Vec<Vec<f64>>,
}

impl MatrixMN {
    /// the function will display the matrix
    pub fn disp(&self) {

        if self.values.is_empty() {
            println!("[ ]");
            return;
        }

        let m: usize = self.values.len();       // number of lines
        let n: usize = self.values[0].len();    // number of columns

        print!("[");

        for i in 0..=(m - 1) {
            for j in 0..=(n - 1) {
                print!("{}", self.values[i][j]);

                if j != n - 1 {
                    print!(", ");
                }
            }

            if i != m - 1 {
                println!(";");
            } else {
                println!("]");
            }
        }

        return;
    }
}
