use std assert

# Parameter name:
# sig type   : any
# name       : closure
# type       : positional
# shape      : closure(any)
# description: the closure to run

# Parameter name:
# sig type   : nothing
# name       : closure
# type       : positional
# shape      : closure(any)
# description: the closure to run


# This is the custom command 1 for explain:

#[test]
def explain_explain_a_command_within_a_closure_1 [] {
  let result = (explain {|| ls | sort-by name type -i | get name } | table -e)
  assert ($result == )
}


