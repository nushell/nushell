use std assert

# Parameter name:
# sig type   : nothing
# name       : name
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


# This is the custom command 1 for export_def:

#[test]
def export_def_define_a_custom_command_in_a_module_and_call_it_1 [] {
  let result = (module spam { export def foo [] { "foo" } }; use spam foo; foo)
  assert ($result == foo)
}


