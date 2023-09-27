use std assert

# Parameter name:
# sig type   : nothing
# name       : module
# type       : positional
# shape      : string
# description: Module or module file

# Parameter name:
# sig type   : nothing
# name       : members
# type       : positional
# shape      : any
# description: Which members of the module to import


# This is the custom command 1 for hide:

#[test]
def hide_hide_the_alias_just_defined_1 [] {
  let result = (alias lll = ls -l; hide lll)
  assert ($result == )
}

# This is the custom command 2 for hide:

#[test]
def hide_hide_a_custom_command_2 [] {
  let result = (def say-hi [] { echo 'Hi!' }; hide say-hi)
  assert ($result == )
}


