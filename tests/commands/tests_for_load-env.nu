use std assert

# Parameter name:
# sig type   : nothing
# name       : update
# type       : positional
# shape      : record
# description: the record to use for updates

# Parameter name:
# sig type   : record
# name       : update
# type       : positional
# shape      : record
# description: the record to use for updates


# This is the custom command 1 for load-env:

#[test]
def load-env_load_variables_from_an_input_stream_1 [] {
  let result = ({NAME: ABE, AGE: UNKNOWN} | load-env; $env.NAME)
  assert ($result == ABE)
}

# This is the custom command 2 for load-env:

#[test]
def load-env_load_variables_from_an_argument_2 [] {
  let result = (load-env {NAME: ABE, AGE: UNKNOWN}; $env.NAME)
  assert ($result == ABE)
}


