use std assert

# Parameter name:
# sig type   : nothing
# name       : name
# type       : rest
# shape      : string
# description: environment variable names to hide

# Parameter name:
# sig type   : nothing
# name       : ignore-errors
# type       : switch
# shape      : 
# description: do not throw an error if an environment variable was not found


# This is the custom command 1 for hide-env:

#[test]
def hide-env_hide_an_environment_variable_1 [] {
  let result = ($env.HZ_ENV_ABC = 1; hide-env HZ_ENV_ABC; 'HZ_ENV_ABC' in (env).name)
  assert ($result == false)
}


