use std assert

# Parameter name:
# sig type   : nothing
# name       : var_name
# type       : positional
# shape      : vardecl
# description: name of the looping variable

# Parameter name:
# sig type   : nothing
# name       : range
# type       : positional
# shape      : "in" any
# description: range of the loop

# Parameter name:
# sig type   : nothing
# name       : block
# type       : positional
# shape      : block
# description: the block to run

# Parameter name:
# sig type   : nothing
# name       : numbered
# type       : switch
# shape      : 
# description: return a numbered item ($it.index and $it.item)


# This is the custom command 1 for for:

#[test]
def for_echo_the_square_of_each_integer_1 [] {
  let result = (for x in [1 2 3] { print ($x * $x) })
  assert ($result == )
}

# This is the custom command 2 for for:

#[test]
def for_work_with_elements_of_a_range_2 [] {
  let result = (for $x in 1..3 { print $x })
  assert ($result == )
}

# This is the custom command 3 for for:

#[test]
def for_number_each_item_and_echo_a_message_3 [] {
  let result = (for $it in ['bob' 'fred'] --numbered { print $"($it.index) is ($it.item)" })
  assert ($result == )
}


