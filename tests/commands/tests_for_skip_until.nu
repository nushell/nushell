use std assert

# Parameter name:
# sig type   : list<any>
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: the predicate that skipped element must not match

# Parameter name:
# sig type   : table
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: the predicate that skipped element must not match


# This is the custom command 1 for skip_until:

#[test]
def skip_until_skip_until_the_element_is_positive_1 [] {
  let result = ([-2 0 2 -1] | skip until {|x| $x > 0 })
  assert ($result == [2, -1])
}

# This is the custom command 2 for skip_until:

#[test]
def skip_until_skip_until_the_element_is_positive_using_stored_condition_2 [] {
  let result = (let cond = {|x| $x > 0 }; [-2 0 2 -1] | skip until $cond)
  assert ($result == [2, -1])
}

# This is the custom command 3 for skip_until:

#[test]
def skip_until_skip_until_the_field_value_is_positive_3 [] {
  let result = ([{a: -2} {a: 0} {a: 2} {a: -1}] | skip until {|x| $x.a > 0 })
  assert ($result == [{a: 2}, {a: -1}])
}


