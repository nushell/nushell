use std assert

# Parameter name:
# sig type   : any
# name       : var_name
# type       : positional
# shape      : vardecl
# description: variable name

# Parameter name:
# sig type   : any
# name       : initial_value
# type       : positional
# shape      : "=" variable
# description: equals sign followed by value


# This is the custom command 1 for let:

#[test]
def let_set_a_variable_to_a_value_1 [] {
  let result = (let x = 10)
  assert ($result == )
}

# This is the custom command 2 for let:

#[test]
def let_set_a_variable_to_the_result_of_an_expression_2 [] {
  let result = (let x = 10 + 100)
  assert ($result == )
}

# This is the custom command 3 for let:

#[test]
def let_set_a_variable_based_on_the_condition_3 [] {
  let result = (let x = if false { -1 } else { 1 })
  assert ($result == )
}


