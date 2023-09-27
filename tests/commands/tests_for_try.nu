use std assert

# Parameter name:
# sig type   : any
# name       : try_block
# type       : positional
# shape      : block
# description: block to run

# Parameter name:
# sig type   : any
# name       : catch_block
# type       : positional
# shape      : "catch" one_of(closure(), closure(any))
# description: block to run if try block fails


# This is the custom command 1 for try:

#[test]
def try_try_to_run_a_missing_command_1 [] {
  let result = (try { asdfasdf })
  assert ($result == )
}

# This is the custom command 2 for try:

#[test]
def try_try_to_run_a_missing_command_2 [] {
  let result = (try { asdfasdf } catch { 'missing' } )
  assert ($result == missing)
}


