use std assert

# Parameter name:
# sig type   : list<any>
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: a closure that must evaluate to a boolean

# Parameter name:
# sig type   : table
# name       : predicate
# type       : positional
# shape      : closure(any, int)
# description: a closure that must evaluate to a boolean


# This is the custom command 1 for any:

#[test]
def any_check_if_any_rows_status_is_the_string_down_1 [] {
  let result = ([[status]; [UP] [DOWN] [UP]] | any {|el| $el.status == DOWN })
  assert ($result == true)
}

# This is the custom command 2 for any:

#[test]
def any_check_that_any_item_is_a_string_2 [] {
  let result = ([1 2 3 4] | any {|| ($in | describe) == 'string' })
  assert ($result == false)
}

# This is the custom command 3 for any:

#[test]
def any_check_if_any_value_is_equal_to_twice_its_own_index_3 [] {
  let result = ([9 8 7 6] | enumerate | any {|i| $i.item == $i.index * 2 })
  assert ($result == true)
}

# This is the custom command 4 for any:

#[test]
def any_check_if_any_of_the_values_are_odd_using_a_stored_closure_4 [] {
  let result = (let cond = {|e| $e mod 2 == 1 }; [2 4 1 6 8] | any $cond)
  assert ($result == true)
}


