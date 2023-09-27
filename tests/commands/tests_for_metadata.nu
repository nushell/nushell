use std assert

# Parameter name:
# sig type   : any
# name       : expression
# type       : positional
# shape      : any
# description: the expression you want metadata for


# This is the custom command 1 for metadata:

#[test]
def metadata_get_the_metadata_of_a_variable_1 [] {
  let result = (let a = 42; metadata $a)
  assert ($result == )
}

# This is the custom command 2 for metadata:

#[test]
def metadata_get_the_metadata_of_the_input_2 [] {
  let result = (ls | metadata)
  assert ($result == )
}


