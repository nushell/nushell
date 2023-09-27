use std assert


# This is the custom command 1 for math_tanh:

#[test]
def math_tanh_apply_the_hyperbolic_tangent_to_10π_1 [] {
  let result = (3.141592 * 10 | math tanh | math round --precision 4)
  assert ($result == 1)
}


