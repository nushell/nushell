use std assert

# Parameter name:
# sig type   : nothing
# name       : module
# type       : positional
# shape      : string
# description: module name or module path

# Parameter name:
# sig type   : nothing
# name       : block
# type       : positional
# shape      : block
# description: body of the module if 'module' parameter is not a path


# This is the custom command 1 for export_module:

#[test]
def export_module_define_a_custom_command_in_a_submodule_of_a_module_and_call_it_1 [] {
  let result = (module spam {
        export module eggs {
            export def foo [] { "foo" }
        }
    }
    use spam eggs
    eggs foo)
  assert ($result == foo)
}


