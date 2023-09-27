use std assert

# Parameter name:
# sig type   : nothing
# name       : range
# type       : positional
# shape      : range
# description: Range of values


# This is the custom command 1 for random_float:

#[test]
def random_float_generate_a_default_float_value_between_0_and_1_1 [] {
  let result = (random float)
  assert ($result == )
}

# This is the custom command 2 for random_float:

#[test]
def random_float_generate_a_random_float_less_than_or_equal_to_500_2 [] {
  let result = (random float ..500)
  assert ($result == )
}

# This is the custom command 3 for random_float:

#[test]
def random_float_generate_a_random_float_greater_than_or_equal_to_100000_3 [] {
  let result = (random float 100000..)
  assert ($result == )
}

# This is the custom command 4 for random_float:

#[test]
def random_float_generate_a_random_float_between_10_and_11_4 [] {
  let result = (random float 1.0..1.1)
  assert ($result == )
}


