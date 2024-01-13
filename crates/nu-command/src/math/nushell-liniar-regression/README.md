# Matrix operation
The structures the store variables in lines and columns:
```rust
pub struct MatrixMN {
    pub values: Vec<Vec<f64>>,
}
```

## Creating a matrix
```rust
let values: Vec<f64> = vec![1.0, 5.0];
let mat: MatrixMN = MatrixMN::create_matrix(&values, 1, 2);    // a vector of f64, nr of lines, nr of columns
```

## Accessing an element
```rust
mat.values[i][j]
```
- element on the line `i - 1` and `j - 1` column
- indexing starts from `0`


# Linear Regression

```rust
let nm1: String = String::from("X-var");
let nm2: String = String::from("Y-var");
let val1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
let val2: Vec<f64> = vec![14.5, 140.1, 201.3, 220.5];

let dt: DataSet = DataSet::new(nm1.clone(), nm2.clone(), val1.clone(), val2.clone());

match dt.compute_linear_regression() {
    Ok(line) => {
        // d : y = a * x + b ; a = slope; b = intercept
        println!("d : y = {} * x + {}", line.slope, line.intercept);
    },
    Err(xbar) => {
        // d : x = constant
        println!("d : x = {}", xbar.x);
    }
}

```


# Explaining The Algorithm
What is linear regression?
The best line that can go through a set of points P1(x1, y1), P2(x2, y2) ... P3(x3, y3)

`d : y = a * x + b (x = ?, y = ?)` = the equation of the line

We are to find the `slope (x)` and the `intercept (y)` of this line.

Assuming that all points are on the same line,
we will obtain the following system of equations:
- a * x1 + b = y1
- a * x2 + b = y2
- a * x3 + b = y3
- .....
- a * xn + b = yn


Therefore, we can define the following matrices
```
    | 1 x1 |                     | y1 |
    | 1 x2 |                     | y2 |
A = | .... |    X = | b |   B =  | .. |
    | 1 xn |        | a |        | yn |
```


Now, the matrix `X` is the answear of the equations,
which are incompatible, therefore they are part of an indeterminate system `A * X = B`.

## How to approximate the matrix X?
```
               A * X = B
At *      |    A * X = B
(At*A)^-1 |    At * A * X = At * B
[(At * A)^(-1) * (At * A)] * X = (At * A)^(-1) * At * B
In * X = (At * A)^(-1) * At * B
X = (At * A)^(-1) * At * B
```


Where:
- `In`:
 1. identity matrix with n lines and n columns
 2. has 1 on the diagonal and 0 in rest
- `At`:
 1. the transposed matrix
 2. columns of the initial matrix become rows of the transpose 
 3. rows of the initial matrix become columns of the transpose 
