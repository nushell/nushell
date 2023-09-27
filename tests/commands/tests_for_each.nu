use std assert

# Parameter name:
# sig type   : any
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: the closure to run

# Parameter name:
# sig type   : any
# name       : keep-empty
# type       : switch
# shape      : 
# description: keep empty result cells

# Parameter name:
# sig type   : list<any>
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: the closure to run

# Parameter name:
# sig type   : list<any>
# name       : keep-empty
# type       : switch
# shape      : 
# description: keep empty result cells

# Parameter name:
# sig type   : table
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: the closure to run

# Parameter name:
# sig type   : table
# name       : keep-empty
# type       : switch
# shape      : 
# description: keep empty result cells


# This is the custom command 1 for each:

#[test]
def each_multiplies_elements_in_the_list_1 [] {
  let result = ([1 2 3] | each {|e| 2 * $e })
  assert ($result == [2, 4, 6])
}

# This is the custom command 2 for each:

#[test]
def each_produce_a_list_of_values_in_the_record_converted_to_string_2 [] {
  let result = ({major:2, minor:1, patch:4} | values | each {|| into string })
  assert ($result == [2, 1, 4])
}

# This is the custom command 3 for each:

#[test]
def each_produce_a_list_that_has_two_for_each_2_in_the_input_3 [] {
  let result = ([1 2 3 2] | each {|e| if $e == 2 { "two" } })
  assert ($result == [two, two])
}

# This is the custom command 4 for each:

#[test]
def each_iterate_over_each_element_producing_a_list_showing_indexes_of_any_2s_4 [] {
  let result = ([1 2 3] | enumerate | each {|e| if $e.item == 2 { $"found 2 at ($e.index)!"} })
  assert ($result == [found 2 at 1!])
}

# This is the custom command 5 for each:

#[test]
def each_iterate_over_each_element_keeping_null_results_5 [] {
  let result = ([1 2 3] | each --keep-empty {|e| if $e == 2 { "found 2!"} })
  assert ($result == [, found 2!, ])
}


