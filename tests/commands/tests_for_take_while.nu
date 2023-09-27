use std assert

# Parameter name:
# sig type   : list<any>
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: the predicate that element(s) must match

# Parameter name:
# sig type   : table
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: the predicate that element(s) must match


# This is the custom command 1 for take_while:

#[test]
def take_while_take_while_the_element_is_negative_1 [] {
  let result = ([-1 -2 9 1] | take while {|x| $x < 0 })
  assert ($result == [-1, -2])
}

# This is the custom command 2 for take_while:

#[test]
def take_while_take_while_the_element_is_negative_using_stored_condition_2 [] {
  let result = (let cond = {|x| $x < 0 }; [-1 -2 9 1] | take while $cond)
  assert ($result == [-1, -2])
}

# This is the custom command 3 for take_while:

#[test]
def take_while_take_while_the_field_value_is_negative_3 [] {
  let result = ([{a: -1} {a: -2} {a: 9} {a: 1}] | take while {|x| $x.a < 0 })
  assert ($result == [{a: -1}, {a: -2}])
}


