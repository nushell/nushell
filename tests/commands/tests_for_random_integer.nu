use std assert

# Parameter name:
# sig type   : nothing
# name       : range
# type       : positional
# shape      : range
# description: Range of values


# This is the custom command 1 for random_integer:

#[test]
def random_integer_generate_an_unconstrained_random_integer_1 [] {
  let result = (random integer)
  assert ($result == )
}

# This is the custom command 2 for random_integer:

#[test]
def random_integer_generate_a_random_integer_less_than_or_equal_to_500_2 [] {
  let result = (random integer ..500)
  assert ($result == )
}

# This is the custom command 3 for random_integer:

#[test]
def random_integer_generate_a_random_integer_greater_than_or_equal_to_100000_3 [] {
  let result = (random integer 100000..)
  assert ($result == )
}

# This is the custom command 4 for random_integer:

#[test]
def random_integer_generate_a_random_integer_between_1_and_10_4 [] {
  let result = (random integer 1..10)
  assert ($result == )
}


