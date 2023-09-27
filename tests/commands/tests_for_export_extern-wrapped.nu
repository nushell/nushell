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


# This is the custom command 1 for export_extern-wrapped:

#[test]
def export_extern-wrapped_export_the_signature_for_an_external_command_1 [] {
  let result = (export extern-wrapped my-echo [...rest] { echo $rest })
  assert ($result == )
}


