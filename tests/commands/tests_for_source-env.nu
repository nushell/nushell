use std assert

# Parameter name:
# sig type   : any
# name       : filename
# type       : positional
# shape      : string
# description: the filepath to the script file to source the environment from


# This is the custom command 1 for source-env:

#[test]
def source-env_sources_the_environment_from_foonu_in_the_current_context_1 [] {
  let result = (source-env foo.nu)
  assert ($result == )
}


