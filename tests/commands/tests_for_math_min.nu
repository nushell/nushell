use std assert


# This is the custom command 1 for math_min:

#[test]
def math_min_compute_the_minimum_of_a_list_of_numbers_1 [] {
  let result = ([-50 100 25] | math min)
  assert ($result == -50)
}

# This is the custom command 2 for math_min:

#[test]
def math_min_compute_the_minima_of_the_columns_of_a_table_2 [] {
  let result = ([{a: 1 b: 3} {a: 2 b: -1}] | math min)
  assert ($result == {a: 1, b: -1})
}

# This is the custom command 3 for math_min:

#[test]
def math_min_find_the_minimum_of_a_list_of_arbitrary_values_warning_weird_3 [] {
  let result = ([-50 'hello' true] | math min)
  assert ($result == true)
}


