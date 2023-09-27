use std assert


# This is the custom command 1 for math_arccosh:

#[test]
def math_arccosh_get_the_arccosh_of_1_1 [] {
  let result = (1 | math arccosh)
  assert ($result == 0)
}


