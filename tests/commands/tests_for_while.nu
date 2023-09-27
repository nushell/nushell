use std assert

# Parameter name:
# sig type   : nothing
# name       : cond
# type       : positional
# shape      : variable
# description: condition to check

# Parameter name:
# sig type   : nothing
# name       : block
# type       : positional
# shape      : block
# description: block to loop if check succeeds


# This is the custom command 1 for while:

#[test]
def while_loop_while_a_condition_is_true_1 [] {
  let result = (mut x = 0; while $x < 10 { $x = $x + 1 })
  assert ($result == )
}


