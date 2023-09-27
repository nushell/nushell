use std assert

# Parameter name:
# sig type   : nothing
# name       : prompt
# type       : positional
# shape      : string
# description: prompt to show the user

# Parameter name:
# sig type   : nothing
# name       : bytes-until-any
# type       : named
# shape      : string
# description: read bytes (not text) until any of the given stop bytes is seen

# Parameter name:
# sig type   : nothing
# name       : numchar
# type       : named
# shape      : int
# description: number of characters to read; suppresses output

# Parameter name:
# sig type   : nothing
# name       : suppress-output
# type       : switch
# shape      : 
# description: don't print keystroke values


# This is the custom command 1 for input:

#[test]
def input_get_input_from_the_user_and_assign_to_a_variable_1 [] {
  let result = (let user_input = (input))
  assert ($result == )
}

# This is the custom command 2 for input:

#[test]
def input_get_two_characters_from_the_user_and_assign_to_a_variable_2 [] {
  let result = (let user_input = (input --numchar 2))
  assert ($result == )
}


