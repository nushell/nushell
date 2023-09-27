use std assert

# Parameter name:
# sig type   : list<any>
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: Predicate closure

# Parameter name:
# sig type   : range
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: Predicate closure

# Parameter name:
# sig type   : table
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: Predicate closure


# This is the custom command 1 for filter:

#[test]
def filter_filter_items_of_a_list_according_to_a_condition_1 [] {
  let result = ([1 2] | filter {|x| $x > 1})
  assert ($result == [2])
}

# This is the custom command 2 for filter:

#[test]
def filter_filter_rows_of_a_table_according_to_a_condition_2 [] {
  let result = ([{a: 1} {a: 2}] | filter {|x| $x.a > 1})
  assert ($result == [{a: 2}])
}

# This is the custom command 3 for filter:

#[test]
def filter_filter_rows_of_a_table_according_to_a_stored_condition_3 [] {
  let result = (let cond = {|x| $x.a > 1}; [{a: 1} {a: 2}] | filter $cond)
  assert ($result == [{a: 2}])
}

# This is the custom command 4 for filter:

#[test]
def filter_filter_items_of_a_range_according_to_a_condition_4 [] {
  let result = (9..13 | filter {|el| $el mod 2 != 0})
  assert ($result == [9, 11, 13])
}

# This is the custom command 5 for filter:

#[test]
def filter_list_all_numbers_above_3_using_an_existing_closure_condition_5 [] {
  let result = (let a = {$in > 3}; [1, 2, 5, 6] | filter $a)
  assert ($result == )
}


