use std assert

# Parameter name:
# sig type   : nothing
# name       : length
# type       : named
# shape      : int
# description: Number of chars


# This is the custom command 1 for random_chars:

#[test]
def random_chars_generate_random_chars_1 [] {
  let result = (random chars)
  assert ($result == )
}

# This is the custom command 2 for random_chars:

#[test]
def random_chars_generate_random_chars_with_specified_length_2 [] {
  let result = (random chars -l 20)
  assert ($result == )
}


