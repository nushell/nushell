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


# This is the custom command 1 for export_use:

#[test]
def export_use_re_export_a_command_from_another_module_1 [] {
  let result = (module spam { export def foo [] { "foo" } }
    module eggs { export use spam foo }
    use eggs foo
    foo
            )
  assert ($result == foo)
}


