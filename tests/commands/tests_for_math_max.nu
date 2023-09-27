use std assert


# This is the custom command 1 for math_max:

#[test]
def math_max_find_the_maximum_of_list_of_numbers_1 [] {
  let result = ([-50 100 25] | math max)
  assert ($result == 100)
}

# This is the custom command 2 for math_max:

#[test]
def math_max_find_the_maxima_of_the_columns_of_a_table_2 [] {
  let result = ([{a: 1 b: 3} {a: 2 b: -1}] | math max)
  assert ($result == {a: 2, b: 3})
}


