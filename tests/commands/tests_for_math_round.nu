use std assert

# Parameter name:
# sig type   : list<number>
# name       : precision
# type       : named
# shape      : number
# description: digits of precision

# Parameter name:
# sig type   : number
# name       : precision
# type       : named
# shape      : number
# description: digits of precision


# This is the custom command 1 for math_round:

#[test]
def math_round_apply_the_round_function_to_a_list_of_numbers_1 [] {
  let result = ([1.5 2.3 -3.1] | math round)
  assert ($result == [2, 2, -3])
}

# This is the custom command 2 for math_round:

#[test]
def math_round_apply_the_round_function_with_precision_specified_2 [] {
  let result = ([1.555 2.333 -3.111] | math round -p 2)
  assert ($result == [1.56, 2.33, -3.11])
}

# This is the custom command 3 for math_round:

#[test]
def math_round_apply_negative_precision_to_a_list_of_numbers_3 [] {
  let result = ([123, 123.3, -123.4] | math round -p -1)
  assert ($result == [120, 120, -120])
}


