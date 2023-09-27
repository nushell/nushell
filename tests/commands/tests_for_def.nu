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
# name       : body
# type       : positional
# shape      : closure()
# description: body of the definition


# This is the custom command 1 for def:

#[test]
def def_define_a_command_and_run_it_1 [] {
  let result = (def say-hi [] { echo 'hi' }; say-hi)
  assert ($result == hi)
}

# This is the custom command 2 for def:

#[test]
def def_define_a_command_and_run_it_with_parameters_2 [] {
  let result = (def say-sth [sth: string] { echo $sth }; say-sth hi)
  assert ($result == hi)
}


