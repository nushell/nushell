use std assert


# This is the custom command 1 for math_floor:

#[test]
def math_floor_apply_the_floor_function_to_a_list_of_numbers_1 [] {
  let result = ([1.5 2.3 -3.1] | math floor)
  assert ($result == [1, 2, -4])
}


