use std assert

# Parameter name:
# sig type   : nothing
# name       : const_name
# type       : positional
# shape      : vardecl
# description: constant name

# Parameter name:
# sig type   : nothing
# name       : initial_value
# type       : positional
# shape      : "=" variable
# description: equals sign followed by constant value


# This is the custom command 1 for const:

#[test]
def const_create_a_new_parse_time_constant_1 [] {
  let result = (const x = 10)
  assert ($result == )
}

# This is the custom command 2 for const:

#[test]
def const_create_a_composite_constant_value_2 [] {
  let result = (const x = { a: 10, b: 20 })
  assert ($result == )
}


