use std assert

# Parameter name:
# sig type   : list<number>
# name       : base
# type       : positional
# shape      : number
# description: Base for which the logarithm should be computed

# Parameter name:
# sig type   : number
# name       : base
# type       : positional
# shape      : number
# description: Base for which the logarithm should be computed


# This is the custom command 1 for math_log:

#[test]
def math_log_get_the_logarithm_of_100_to_the_base_10_1 [] {
  let result = (100 | math log 10)
  assert ($result == 2)
}

# This is the custom command 2 for math_log:

#[test]
def math_log_get_the_log2_of_a_list_of_values_2 [] {
  let result = ([16 8 4] | math log 2)
  assert ($result == [4, 3, 2])
}


