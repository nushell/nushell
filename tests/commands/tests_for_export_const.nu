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


# This is the custom command 1 for export_const:

#[test]
def export_const_re_export_a_command_from_another_module_1 [] {
  let result = (module spam { export const foo = 3; }
    module eggs { export use spam foo }
    use eggs foo
    foo
            )
  assert ($result == 3)
}


