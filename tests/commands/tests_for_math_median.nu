use std assert


# This is the custom command 1 for math_median:

#[test]
def math_median_compute_the_median_of_a_list_of_numbers_1 [] {
  let result = ([3 8 9 12 12 15] | math median)
  assert ($result == 10.5)
}

# This is the custom command 2 for math_median:

#[test]
def math_median_compute_the_medians_of_the_columns_of_a_table_2 [] {
  let result = ([{a: 1 b: 3} {a: 2 b: -1} {a: -3 b: 5}] | math median)
  assert ($result == {a: 1, b: 3})
}


