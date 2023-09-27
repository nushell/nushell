use std assert

# Parameter name:
# sig type   : nothing
# name       : block
# type       : positional
# shape      : block
# description: the block to run to set the environment


# This is the custom command 1 for export-env:

#[test]
def export-env_set_an_environment_variable_1 [] {
  let result = (export-env { $env.SPAM = 'eggs' })
  assert ($result == )
}

# This is the custom command 2 for export-env:

#[test]
def export-env_set_an_environment_variable_and_examine_its_value_2 [] {
  let result = (export-env { $env.SPAM = 'eggs' }; $env.SPAM)
  assert ($result == eggs)
}


