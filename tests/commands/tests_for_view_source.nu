use std assert

# Parameter name:
# sig type   : nothing
# name       : item
# type       : positional
# shape      : any
# description: name or block to view


# This is the custom command 1 for view_source:

#[test]
def view_source_view_the_source_of_a_code_block_1 [] {
  let result = (let abc = {|| echo 'hi' }; view source $abc)
  assert ($result == {|| echo 'hi' })
}

# This is the custom command 2 for view_source:

#[test]
def view_source_view_the_source_of_a_custom_command_2 [] {
  let result = (def hi [] { echo 'Hi!' }; view source hi)
  assert ($result == def hi [] { echo 'Hi!' })
}

# This is the custom command 3 for view_source:

#[test]
def view_source_view_the_source_of_a_custom_command_which_participates_in_the_caller_environment_3 [] {
  let result = (def-env foo [] { $env.BAR = 'BAZ' }; view source foo)
  assert ($result == def foo [] { $env.BAR = 'BAZ' })
}

# This is the custom command 4 for view_source:

#[test]
def view_source_view_the_source_of_a_custom_command_with_flags_and_arguments_4 [] {
  let result = (def test [a?:any --b:int ...rest:string] { echo 'test' }; view source test)
  assert ($result == def test [ a?: any --b: int ...rest: string] { echo 'test' })
}

# This is the custom command 5 for view_source:

#[test]
def view_source_view_the_source_of_a_module_5 [] {
  let result = (module mod-foo { export-env { $env.FOO_ENV = 'BAZ' } }; view source mod-foo)
  assert ($result ==  export-env { $env.FOO_ENV = 'BAZ' })
}

# This is the custom command 6 for view_source:

#[test]
def view_source_view_the_source_of_an_alias_6 [] {
  let result = (alias hello = echo hi; view source hello)
  assert ($result == echo hi)
}


