use std assert

# Parameter name:
# sig type   : any
# name       : closure
# type       : positional
# shape      : closure(any)
# description: the closure to run

# Parameter name:
# sig type   : any
# name       : source
# type       : switch
# shape      : 
# description: Collect source code in the report

# Parameter name:
# sig type   : any
# name       : values
# type       : switch
# shape      : 
# description: Collect values in the report

# Parameter name:
# sig type   : any
# name       : max-depth
# type       : named
# shape      : int
# description: How many levels of blocks to step into (default: 1)


# This is the custom command 1 for profile:

#[test]
def profile_profile_some_code_stepping_into_the_spam_command_and_collecting_source_1 [] {
  let result = (def spam [] { "spam" }; profile {|| spam | str length } -d 2 --source)
  assert ($result == )
}


