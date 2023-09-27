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


# This is the custom command 1 for extern:

#[test]
def extern_write_a_signature_for_an_external_command_1 [] {
  let result = (extern echo [text: string])
  assert ($result == )
}


