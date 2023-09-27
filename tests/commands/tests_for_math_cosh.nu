use std assert


# This is the custom command 1 for math_cosh:

#[test]
def math_cosh_apply_the_hyperbolic_cosine_to_1_1 [] {
  let result = (1 | math cosh)
  assert ($result == 1.5430806348152435)
}


