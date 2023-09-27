use std assert

# Parameter name:
# sig type   : list<any>
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: the predicate that element(s) must not match

# Parameter name:
# sig type   : table
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: the predicate that element(s) must not match


# This is the custom command 1 for take_until:

#[test]
def take_until_take_until_the_element_is_positive_1 [] {
  let result = ([-1 -2 9 1] | take until {|x| $x > 0 })
  assert ($result == [-1, -2])
}

# This is the custom command 2 for take_until:

#[test]
def take_until_take_until_the_element_is_positive_using_stored_condition_2 [] {
  let result = (let cond = {|x| $x > 0 }; [-1 -2 9 1] | take until $cond)
  assert ($result == [-1, -2])
}

# This is the custom command 3 for take_until:

#[test]
def take_until_take_until_the_field_value_is_positive_3 [] {
  let result = ([{a: -1} {a: -2} {a: 9} {a: 1}] | take until {|x| $x.a > 0 })
  assert ($result == [{a: -1}, {a: -2}])
}


