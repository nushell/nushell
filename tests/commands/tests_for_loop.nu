use std assert

# Parameter name:
# sig type   : nothing
# name       : block
# type       : positional
# shape      : block
# description: block to loop


# This is the custom command 1 for loop:

#[test]
def loop_loop_while_a_condition_is_true_1 [] {
  let result = (mut x = 0; loop { if $x > 10 { break }; $x = $x + 1 }; $x)
  assert ($result == 11)
}


