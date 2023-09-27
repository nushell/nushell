use std assert

# Parameter name:
# sig type   : nothing
# name       : return_value
# type       : positional
# shape      : any
# description: optional value to return


# This is the custom command 1 for return:

#[test]
def return_return_early_1 [] {
  let result = (def foo [] { return })
  assert ($result == )
}


