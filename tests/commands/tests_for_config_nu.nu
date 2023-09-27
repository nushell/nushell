use std assert

# Parameter name:
# sig type   : nothing
# name       : default
# type       : switch
# shape      : 
# description: Print default `config.nu` file instead.


# This is the custom command 1 for config_nu:

#[test]
def config_nu_allow_user_to_open_and_update_nu_config_1 [] {
  let result = (config nu)
  assert ($result == )
}

# This is the custom command 2 for config_nu:

#[test]
def config_nu_allow_user_to_print_default_confignu_file_2 [] {
  let result = (config nu --default,)
  assert ($result == )
}

# This is the custom command 3 for config_nu:

#[test]
def config_nu_allow_saving_the_default_confignu_locally_3 [] {
  let result = (config nu --default | save -f ~/.config/nushell/default_config.nu)
  assert ($result == )
}


