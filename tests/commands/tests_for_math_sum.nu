use std assert


# This is the custom command 1 for math_sum:

#[test]
def math_sum_sum_a_list_of_numbers_1 [] {
  let result = ([1 2 3] | math sum)
  assert ($result == 6)
}

# This is the custom command 2 for math_sum:

#[test]
def math_sum_get_the_disk_usage_for_the_current_directory_2 [] {
  let result = (ls | get size | math sum)
  assert ($result == )
}


