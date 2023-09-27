use std assert

# Parameter name:
# sig type   : any
# name       : command
# type       : positional
# shape      : one_of(block, expression)
# description: the command or block to run

# Parameter name:
# sig type   : nothing
# name       : command
# type       : positional
# shape      : one_of(block, expression)
# description: the command or block to run


# This is the custom command 1 for timeit:

#[test]
def timeit_times_a_command_within_a_closure_1 [] {
  let result = (timeit { sleep 500ms })
  assert ($result == )
}

# This is the custom command 2 for timeit:

#[test]
def timeit_times_a_command_using_an_existing_input_2 [] {
  let result = (http get https://www.nushell.sh/book/ | timeit { split chars })
  assert ($result == )
}

# This is the custom command 3 for timeit:

#[test]
def timeit_times_a_command_invocation_3 [] {
  let result = (timeit ls -la)
  assert ($result == )
}


