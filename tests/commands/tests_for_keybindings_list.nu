use std assert

# Parameter name:
# sig type   : nothing
# name       : modifiers
# type       : switch
# shape      : 
# description: list of modifiers

# Parameter name:
# sig type   : nothing
# name       : keycodes
# type       : switch
# shape      : 
# description: list of keycodes

# Parameter name:
# sig type   : nothing
# name       : modes
# type       : switch
# shape      : 
# description: list of edit modes

# Parameter name:
# sig type   : nothing
# name       : events
# type       : switch
# shape      : 
# description: list of reedline event

# Parameter name:
# sig type   : nothing
# name       : edits
# type       : switch
# shape      : 
# description: list of edit commands


# This is the custom command 1 for keybindings_list:

#[test]
def keybindings_list_get_list_of_key_modifiers_1 [] {
  let result = (keybindings list -m)
  assert ($result == )
}

# This is the custom command 2 for keybindings_list:

#[test]
def keybindings_list_get_list_of_reedline_events_and_edit_commands_2 [] {
  let result = (keybindings list -e -d)
  assert ($result == )
}

# This is the custom command 3 for keybindings_list:

#[test]
def keybindings_list_get_list_with_all_the_available_options_3 [] {
  let result = (keybindings list)
  assert ($result == )
}


