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


# This is the custom command 1 for mut:

#[test]
def mut_set_a_mutable_variable_to_a_value_then_update_it_1 [] {
  let result = (mut x = 10; $x = 12)
  assert ($result == )
}

# This is the custom command 2 for mut:

#[test]
def mut_upsert_a_value_inside_a_mutable_data_structure_2 [] {
  let result = (mut a = {b:{c:1}}; $a.b.c = 2)
  assert ($result == )
}

# This is the custom command 3 for mut:

#[test]
def mut_set_a_mutable_variable_to_the_result_of_an_expression_3 [] {
  let result = (mut x = 10 + 100)
  assert ($result == )
}

# This is the custom command 4 for mut:

#[test]
def mut_set_a_mutable_variable_based_on_the_condition_4 [] {
  let result = (mut x = if false { -1 } else { 1 })
  assert ($result == )
}


