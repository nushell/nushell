use std assert

# Parameter name:
# sig type   : nothing
# name       : clear
# type       : switch
# shape      : 
# description: Clears out the history entries

# Parameter name:
# sig type   : nothing
# name       : long
# type       : switch
# shape      : 
# description: Show long listing of entries for sqlite history


# This is the custom command 1 for history:

#[test]
def history_get_current_history_length_1 [] {
  let result = (history | length)
  assert ($result == )
}

# This is the custom command 2 for history:

#[test]
def history_show_last_5_commands_you_have_ran_2 [] {
  let result = (history | last 5)
  assert ($result == )
}

# This is the custom command 3 for history:

#[test]
def history_search_all_the_commands_from_history_that_contains_cargo_3 [] {
  let result = (history | where command =~ cargo | get command)
  assert ($result == )
}


