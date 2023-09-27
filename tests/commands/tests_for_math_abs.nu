use std assert


# This is the custom command 1 for math_abs:

#[test]
def math_abs_compute_absolute_value_of_each_number_in_a_list_of_numbers_1 [] {
  let result = ([-50 -100.0 25] | math abs)
  assert ($result == [50, 100, 25])
}


