use std assert

# Parameter name:
# sig type   : nothing
# name       : def_name
# type       : positional
# shape      : string
# description: definition name

# Parameter name:
# sig type   : nothing
# name       : params
# type       : positional
# shape      : signature
# description: parameters

# Parameter name:
# sig type   : nothing
# name       : block
# type       : positional
# shape      : block
# description: body of the definition


# This is the custom command 1 for def-env:

#[test]
def def-env_set_environment_variable_by_call_a_custom_command_1 [] {
  let result = (def-env foo [] { $env.BAR = "BAZ" }; foo; $env.BAR)
  assert ($result == BAZ)
}


