use std assert

# Parameter name:
# sig type   : record
# name       : closure
# type       : positional
# shape      : closure(any, any)
# description: the closure to run


# This is the custom command 1 for items:

#[test]
def items_iterate_over_each_key_value_pair_of_a_record_1 [] {
  let result = ({ new: york, san: francisco } | items {|key, value| echo $'($key) ($value)' })
  assert ($result == [new york, san francisco])
}


