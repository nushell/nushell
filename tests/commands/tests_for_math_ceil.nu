use std assert


# This is the custom command 1 for math_ceil:

#[test]
def math_ceil_apply_the_ceil_function_to_a_list_of_numbers_1 [] {
  let result = ([1.5 2.3 -3.1] | math ceil)
  assert ($result == [2, 3, -3])
}


