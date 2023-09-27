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
# shape      : block
# description: wrapper code block


# This is the custom command 1 for extern-wrapped:

#[test]
def extern-wrapped_define_a_custom_wrapper_for_an_external_command_1 [] {
  let result = (extern-wrapped my-echo [...rest] { echo $rest }; my-echo spam)
  assert ($result == [spam])
}


