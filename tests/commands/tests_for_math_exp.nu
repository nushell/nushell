use std assert


# This is the custom command 1 for math_exp:

#[test]
def math_exp_get_e_raised_to_the_power_of_zero_1 [] {
  let result = (0 | math exp)
  assert ($result == 1)
}

# This is the custom command 2 for math_exp:

#[test]
def math_exp_get_e_same_as_math_e_2 [] {
  let result = (1 | math exp)
  assert ($result == 2.718281828459045)
}


