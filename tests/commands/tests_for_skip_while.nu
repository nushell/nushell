use std assert

# Parameter name:
# sig type   : list<any>
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: the predicate that skipped element must match

# Parameter name:
# sig type   : table
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: the predicate that skipped element must match


# This is the custom command 1 for skip_while:

#[test]
def skip_while_skip_while_the_element_is_negative_1 [] {
  let result = ([-2 0 2 -1] | skip while {|x| $x < 0 })
  assert ($result == [0, 2, -1])
}

# This is the custom command 2 for skip_while:

#[test]
def skip_while_skip_while_the_element_is_negative_using_stored_condition_2 [] {
  let result = (let cond = {|x| $x < 0 }; [-2 0 2 -1] | skip while $cond)
  assert ($result == [0, 2, -1])
}

# This is the custom command 3 for skip_while:

#[test]
def skip_while_skip_while_the_field_value_is_negative_3 [] {
  let result = ([{a: -2} {a: 0} {a: 2} {a: -1}] | skip while {|x| $x.a < 0 })
  assert ($result == [{a: 0}, {a: 2}, {a: -1}])
}


