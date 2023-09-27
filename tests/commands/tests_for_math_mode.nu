use std assert


# This is the custom command 1 for math_mode:

#[test]
def math_mode_compute_the_modes_of_a_list_of_numbers_1 [] {
  let result = ([3 3 9 12 12 15] | math mode)
  assert ($result == [3, 12])
}

# This is the custom command 2 for math_mode:

#[test]
def math_mode_compute_the_modes_of_the_columns_of_a_table_2 [] {
  let result = ([{a: 1 b: 3} {a: 2 b: -1} {a: 1 b: 5}] | math mode)
  assert ($result == {a: [1], b: [-1, 3, 5]})
}


