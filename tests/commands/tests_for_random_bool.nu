use std assert

# Parameter name:
# sig type   : nothing
# name       : bias
# type       : named
# shape      : number
# description: Adjusts the probability of a "true" outcome


# This is the custom command 1 for random_bool:

#[test]
def random_bool_generate_a_random_boolean_value_1 [] {
  let result = (random bool)
  assert ($result == )
}

# This is the custom command 2 for random_bool:

#[test]
def random_bool_generate_a_random_boolean_value_with_a_75_chance_of_true_2 [] {
  let result = (random bool --bias 0.75)
  assert ($result == )
}


