use std assert

# Parameter name:
# sig type   : nothing
# name       : application
# type       : positional
# shape      : string
# description: application

# Parameter name:
# sig type   : nothing
# name       : all
# type       : switch
# shape      : 
# description: list all executables


# This is the custom command 1 for which:

#[test]
def which_find_if_the_myapp_application_is_available_1 [] {
  let result = (which myapp)
  assert ($result == )
}


