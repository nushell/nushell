use std assert

# Parameter name:
# sig type   : any
# name       : cond
# type       : positional
# shape      : variable
# description: condition to check

# Parameter name:
# sig type   : any
# name       : then_block
# type       : positional
# shape      : block
# description: block to run if check succeeds

# Parameter name:
# sig type   : any
# name       : else_expression
# type       : positional
# shape      : "else" one_of(block, expression)
# description: expression or block to run if check fails


# This is the custom command 1 for if:

#[test]
def if_output_a_value_if_a_condition_matches_otherwise_return_nothing_1 [] {
  let result = (if 2 < 3 { 'yes!' })
  assert ($result == yes!)
}

# This is the custom command 2 for if:

#[test]
def if_output_a_value_if_a_condition_matches_else_return_another_value_2 [] {
  let result = (if 5 < 3 { 'yes!' } else { 'no!' })
  assert ($result == no!)
}

# This is the custom command 3 for if:

#[test]
def if_chain_multiple_ifs_together_3 [] {
  let result = (if 5 < 3 { 'yes!' } else if 4 < 5 { 'no!' } else { 'okay!' })
  assert ($result == no!)
}


