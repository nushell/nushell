use std assert


# This is the custom command 1 for ignore:

#[test]
def ignore_ignore_the_output_of_an_echo_command_1 [] {
  let result = (echo done | ignore)
  assert ($result == )
}


