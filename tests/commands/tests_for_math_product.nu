use std assert


# This is the custom command 1 for math_product:

#[test]
def math_product_compute_the_product_of_a_list_of_numbers_1 [] {
  let result = ([2 3 3 4] | math product)
  assert ($result == 72)
}


