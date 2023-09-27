use std assert


# This is the custom command 1 for math_sqrt:

#[test]
def math_sqrt_compute_the_square_root_of_each_number_in_a_list_1 [] {
  let result = ([9 16] | math sqrt)
  assert ($result == [3, 4])
}


