use std assert

# Parameter name:
# sig type   : list<any>
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: the closure to run

# Parameter name:
# sig type   : table
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: the closure to run


# This is the custom command 1 for each_while:

#[test]
def each_while_produces_a_list_of_each_element_before_the_3_doubled_1 [] {
  let result = ([1 2 3 2 1] | each while {|e| if $e < 3 { $e * 2 } })
  assert ($result == [2, 4])
}

# This is the custom command 2 for each_while:

#[test]
def each_while_output_elements_until_reaching_stop_2 [] {
  let result = ([1 2 stop 3 4] | each while {|e| if $e != 'stop' { $"Output: ($e)" } })
  assert ($result == [Output: 1, Output: 2])
}

# This is the custom command 3 for each_while:

#[test]
def each_while_iterate_over_each_element_printing_the_matching_value_and_its_index_3 [] {
  let result = ([1 2 3] | enumerate | each while {|e| if $e.item < 2 { $"value ($e.item) at ($e.index)!"} })
  assert ($result == [value 1 at 0!])
}


