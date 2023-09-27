use std assert

# Parameter name:
# sig type   : any
# name       : closure
# type       : positional
# shape      : closure(any)
# description: the closure to run once the stream is collected

# Parameter name:
# sig type   : any
# name       : keep-env
# type       : switch
# shape      : 
# description: let the block affect environment variables


# This is the custom command 1 for collect:

#[test]
def collect_use_the_second_value_in_the_stream_1 [] {
  let result = ([1 2 3] | collect { |x| $x.1 })
  assert ($result == 2)
}


