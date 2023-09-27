use std assert


# This is the custom command 1 for complete:

#[test]
def complete_run_the_external_command_to_completion_capturing_stdout_and_exit_code_1 [] {
  let result = (^external arg1 | complete)
  assert ($result == )
}

# This is the custom command 2 for complete:

#[test]
def complete_run_external_command_to_completion_capturing_stdout_stderr_and_exit_code_2 [] {
  let result = (do { ^external arg1 } | complete)
  assert ($result == )
}


