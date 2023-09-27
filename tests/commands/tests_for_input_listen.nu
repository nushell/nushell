use std assert

# Parameter name:
# sig type   : nothing
# name       : types
# type       : named
# shape      : list<string>
# description: Listen for event of specified types only (can be one of: focus, key, mouse, paste, resize)

# Parameter name:
# sig type   : nothing
# name       : raw
# type       : switch
# shape      : 
# description: Add raw_code field with numeric value of keycode and raw_flags with bit mask flags


# This is the custom command 1 for input_listen:

#[test]
def input_listen_listen_for_a_keyboard_shortcut_and_find_out_how_nu_receives_it_1 [] {
  let result = (input listen --types [key])
  assert ($result == )
}


