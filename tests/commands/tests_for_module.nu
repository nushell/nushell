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
# description: body of the module if 'module' parameter is not a module path


# This is the custom command 1 for module:

#[test]
def module_define_a_custom_command_in_a_module_and_call_it_1 [] {
  let result = (module spam { export def foo [] { "foo" } }; use spam foo; foo)
  assert ($result == foo)
}

# This is the custom command 2 for module:

#[test]
def module_define_an_environment_variable_in_a_module_2 [] {
  let result = (module foo { export-env { $env.FOO = "BAZ" } }; use foo; $env.FOO)
  assert ($result == BAZ)
}

# This is the custom command 3 for module:

#[test]
def module_define_a_custom_command_that_participates_in_the_environment_in_a_module_and_call_it_3 [] {
  let result = (module foo { export def-env bar [] { $env.FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR)
  assert ($result == BAZ)
}


