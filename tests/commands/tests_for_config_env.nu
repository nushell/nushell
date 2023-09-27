use std assert

# Parameter name:
# sig type   : nothing
# name       : default
# type       : switch
# shape      : 
# description: Print default `env.nu` file instead.


# This is the custom command 1 for config_env:

#[test]
def config_env_allow_user_to_open_and_update_nu_env_1 [] {
  let result = (config env)
  assert ($result == )
}

# This is the custom command 2 for config_env:

#[test]
def config_env_allow_user_to_print_default_envnu_file_2 [] {
  let result = (config env --default,)
  assert ($result == )
}

# This is the custom command 3 for config_env:

#[test]
def config_env_allow_saving_the_default_envnu_locally_3 [] {
  let result = (config env --default | save -f ~/.config/nushell/default_env.nu)
  assert ($result == )
}


