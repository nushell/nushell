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
# type       : rest
# shape      : any
# description: Which members of the module to import


# This is the custom command 1 for use:

#[test]
def use_define_a_custom_command_in_a_module_and_call_it_1 [] {
  let result = (module spam { export def foo [] { "foo" } }; use spam foo; foo)
  assert ($result == foo)
}

# This is the custom command 2 for use:

#[test]
def use_define_a_custom_command_that_participates_in_the_environment_in_a_module_and_call_it_2 [] {
  let result = (module foo { export def-env bar [] { $env.FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR)
  assert ($result == BAZ)
}

# This is the custom command 3 for use:

#[test]
def use_use_a_plain_module_name_to_import_its_definitions_qualified_by_the_module_name_3 [] {
  let result = (module spam { export def foo [] { "foo" }; export def bar [] { "bar" } }; use spam; (spam foo) + (spam bar))
  assert ($result == foobar)
}

# This is the custom command 4 for use:

#[test]
def use_specify__to_use_all_definitions_in_a_module_4 [] {
  let result = (module spam { export def foo [] { "foo" }; export def bar [] { "bar" } }; use spam *; (foo) + (bar))
  assert ($result == foobar)
}

# This is the custom command 5 for use:

#[test]
def use_to_use_commands_with_spaces_like_subcommands_surround_them_with_quotes_5 [] {
  let result = (module spam { export def 'foo bar' [] { "baz" } }; use spam 'foo bar'; foo bar)
  assert ($result == baz)
}

# This is the custom command 6 for use:

#[test]
def use_to_use_multiple_definitions_from_a_module_wrap_them_in_a_list_6 [] {
  let result = (module spam { export def foo [] { "foo" }; export def 'foo bar' [] { "baz" } }; use spam ['foo', 'foo bar']; (foo) + (foo bar))
  assert ($result == foobaz)
}


