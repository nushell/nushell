use std assert

# Parameter name:
# sig type   : any
# name       : filename
# type       : positional
# shape      : path
# description: the filepath to the script file to source


# This is the custom command 1 for source:

#[test]
def source_runs_foonu_in_the_current_context_1 [] {
  let result = (source foo.nu)
  assert ($result == )
}

# This is the custom command 2 for source:

#[test]
def source_runs_foonu_in_current_context_and_call_the_command_defined_suppose_foonu_has_content_def_say_hi___echo_hi__2 [] {
  let result = (source ./foo.nu; say-hi)
  assert ($result == )
}


