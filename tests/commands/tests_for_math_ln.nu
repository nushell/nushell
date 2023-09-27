use std assert


# This is the custom command 1 for math_ln:

#[test]
def math_ln_get_the_natural_logarithm_of_e_1 [] {
  let result = (2.7182818 | math ln | math round --precision 4)
  assert ($result == 1)
}


