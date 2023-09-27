use std assert


# This is the custom command 1 for math_avg:

#[test]
def math_avg_compute_the_average_of_a_list_of_numbers_1 [] {
  let result = ([-50 100.0 25] | math avg)
  assert ($result == 25)
}


