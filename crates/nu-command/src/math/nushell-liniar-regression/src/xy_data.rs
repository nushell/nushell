use crate::matrix::MatrixMN;

pub struct DataSet {
    pub x_name: String,
    pub y_name: String,
    pub x_values: Vec<f64>,
    pub y_values: Vec<f64>,
}


/// d : x = a
pub struct XBar {
    pub x: f64,
}


/// d : y = a * x + b
/// a = slope
/// b = intercept
pub struct Line {
    pub slope: f64,
    pub intercept: f64,
}


impl DataSet {
    pub fn validate_data_set(&self) {
        if self.x_name.is_empty() {
            panic!("Missing name of the input variable.");
        }
        if self.y_name.is_empty() {
            panic!("Missing name of the output variable.");
        }
        if self.x_values.is_empty() {
            panic!("The are no values for the X-axis.");
        }
        if self.y_values.is_empty() {
            panic!("The are no values for the Y-axis.");
        }
        if self.x_values.len() != self.y_values.len() {
            panic!("Different number of elements for the input and output values.");
        }
    }
}

impl DataSet {
    pub fn new(name1: String, name2: String, val1: Vec<f64>, val2: Vec<f64>) -> Self {
        let ret: DataSet =  DataSet {
            x_name: name1,
            y_name: name2,
            x_values: val1,
            y_values: val2,
        };

        ret.validate_data_set();
        return ret;
    }
}


impl DataSet {
    /// indexing starts from 0
    pub fn new_from_vec(val2: Vec<f64>) -> Self {
        let mut val1: Vec<f64> = Vec::new();

        for i in 0..=(val2.len() - 1) {
            val1.push(i as f64);
        }

        let ret: DataSet = DataSet {
            x_name: String::from("X"),
            y_name: String::from("Y"),
            x_values: val1,
            y_values: val2,
        };

        ret.validate_data_set();
        return ret;
    }
}


pub fn avg(vector: &Vec<f64>) -> f64 {
    if vector.is_empty() {
        return 0.0f64;
    }

    let mut sum: f64 = 0.0f64;
    for el in vector {
        sum += el;
    }

    return sum / (vector.len() as f64);
}


impl DataSet {
    pub fn print_horizontally(&self) {
        self.validate_data_set();
        println!("{}: {:?}", self.x_name, self.x_values);
        println!("{}: {:?}", self.y_name, self.y_values);
    }

    pub fn print_vertically(&self) {
        self.validate_data_set();
        println!("X = {}, Y = {}", self.x_name, self.y_name);
        for i in 0..self.x_values.len() {
            println!("{}, {}", self.x_values[i], self.y_values[i]);
        }
    }
}

impl DataSet {
    /// Result<Ok, Err>
    /// Err -> the equation of a vertical line
    /// Ok -> the equation of a (oblique / horizontal) line
    pub fn compute_linear_regression(&self) -> Result<Line, XBar> {
        self.validate_data_set();

        let len: usize = self.x_values.len();
        let mut a: MatrixMN = MatrixMN::ones(len, 2);
        let mut b: MatrixMN = MatrixMN::ones(len, 1);

        for i in 0..=(len - 1) {
            a.values[i][1] = self.x_values[i];
            b.values[i][0] = self.y_values[i];
        }

        let stage1: MatrixMN = MatrixMN::mul(&a.transpose(), &a);


        if !stage1.is_invertible() {
            let ret = XBar {
                x: avg(&self.x_values),
            };
            return Err(ret);
        }

        let stage2: MatrixMN = stage1.inverse();
        let stage3: MatrixMN = MatrixMN::mul(&stage2, &a.transpose());
        let stage4: MatrixMN = MatrixMN::mul(&stage3, &b);

        let ret = Line {
            slope: stage4.values[1][0],
            intercept: stage4.values[0][0],
        };

        return Ok(ret);
    }
}


impl DataSet {
    pub fn equation_linear_regression(&self) -> String {
        match self.compute_linear_regression() {
            Ok(line) => {
                return display_line(line);
            },
            Err(bar) => {
                return format!("d : x = {:.10}", bar.x);
            }
        }
    }
}


fn display_line(line: Line) -> String {
    // d : y = a * x + b
    let a: f64 = line.slope;
    let b: f64 = line.intercept;

    let tolerance: f64 = 1e-10; // comparison with 10 decimals

    if a.abs() < tolerance && b.abs() < tolerance {   // d : y = 0
        return format!("d : y = {:.10}", 0);
    }


    if a.abs() < tolerance && b.abs() < tolerance {    // d : y = b
        return format!("d : y = -{:.10}", -b);
    }

    if a.abs() < tolerance && b > 0.0 {    // d : y = b
        return format!("d : y = {:.10}", b);
    }


    if a != 0.0 && b.abs() < tolerance {   // d : y = a * x
        return format!("d : y = {:.10} * x", a);
    }

    // d : y = a * x + b
    if b < 0.0 {
        return format!("d : y = {:.10} * x - {:.10}", a, -b);
    }
    return format!("d : y = {:.10} * x + {:.10}", a, b);
}


impl DataSet {
    pub fn display_variables(&self) {
        println!("\"{}\": {:?}", self.x_name, self.x_values);
        println!("\"{}\": {:?}", self.y_name, self.y_values);
    }
}
