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


# This is the custom command 1 for all:

#[test]
def all_check_if_each_rows_status_is_the_string_up_1 [] {
  let result = ([[status]; [UP] [UP]] | all {|el| $el.status == UP })
  assert ($result == true)
}

# This is the custom command 2 for all:

#[test]
def all_check_that_each_item_is_a_string_2 [] {
  let result = ([foo bar 2 baz] | all {|| ($in | describe) == 'string' })
  assert ($result == false)
}

# This is the custom command 3 for all:

#[test]
def all_check_that_all_values_are_equal_to_twice_their_index_3 [] {
  let result = ([0 2 4 6] | enumerate | all {|i| $i.item == $i.index * 2 })
  assert ($result == true)
}

# This is the custom command 4 for all:

#[test]
def all_check_that_all_of_the_values_are_even_using_a_stored_closure_4 [] {
  let result = (let cond = {|el| ($el mod 2) == 0 }; [2 4 6 8] | all $cond)
  assert ($result == true)
}


