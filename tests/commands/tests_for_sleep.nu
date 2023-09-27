use std assert

# Parameter name:
# sig type   : nothing
# name       : duration
# type       : positional
# shape      : duration
# description: time to sleep


# This is the custom command 1 for sleep:

#[test]
def sleep_sleep_for_1sec_1 [] {
  let result = (sleep 1sec)
  assert ($result == )
}

# This is the custom command 2 for sleep:

#[test]
def sleep_sleep_for_3sec_2 [] {
  let result = (sleep 1sec 1sec 1sec)
  assert ($result == )
}

# This is the custom command 3 for sleep:

#[test]
def sleep_send_output_after_1sec_3 [] {
  let result = (sleep 1sec; echo done)
  assert ($result == )
}


