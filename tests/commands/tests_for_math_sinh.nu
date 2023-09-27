use std assert


# This is the custom command 1 for math_sinh:

#[test]
def math_sinh_apply_the_hyperbolic_sine_to_1_1 [] {
  let result = (1 | math sinh)
  assert ($result == 1.1752011936438014)
}


