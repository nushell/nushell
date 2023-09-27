use std assert

# Parameter name:
# sig type   : list<any>
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: the closure to run

# Parameter name:
# sig type   : list<any>
# name       : threads
# type       : named
# shape      : int
# description: the number of threads to use

# Parameter name:
# sig type   : list<any>
# name       : keep-order
# type       : switch
# shape      : 
# description: keep sequence of output same as the order of input

# Parameter name:
# sig type   : range
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: the closure to run

# Parameter name:
# sig type   : range
# name       : threads
# type       : named
# shape      : int
# description: the number of threads to use

# Parameter name:
# sig type   : range
# name       : keep-order
# type       : switch
# shape      : 
# description: keep sequence of output same as the order of input

# Parameter name:
# sig type   : table
# name       : closure
# type       : positional
# shape      : closure(any, int)
# description: the closure to run

# Parameter name:
# sig type   : table
# name       : threads
# type       : named
# shape      : int
# description: the number of threads to use

# Parameter name:
# sig type   : table
# name       : keep-order
# type       : switch
# shape      : 
# description: keep sequence of output same as the order of input


# This is the custom command 1 for par-each:

#[test]
def par-each_multiplies_each_number_note_that_the_list_will_become_arbitrarily_disordered_1 [] {
  let result = ([1 2 3] | par-each {|e| $e * 2 })
  assert ($result == )
}

# This is the custom command 2 for par-each:

#[test]
def par-each_multiplies_each_number_keeping_an_original_order_2 [] {
  let result = ([1 2 3] | par-each --keep-order {|e| $e * 2 })
  assert ($result == [2, 4, 6])
}

# This is the custom command 3 for par-each:

#[test]
def par-each_enumerate_and_sort_by_can_be_used_to_reconstruct_the_original_order_3 [] {
  let result = (1..3 | enumerate | par-each {|p| update item ($p.item * 2)} | sort-by item | get item)
  assert ($result == [2, 4, 6])
}

# This is the custom command 4 for par-each:

#[test]
def par-each_output_can_still_be_sorted_afterward_4 [] {
  let result = ([foo bar baz] | par-each {|e| $e + '!' } | sort)
  assert ($result == [bar!, baz!, foo!])
}

# This is the custom command 5 for par-each:

#[test]
def par-each_iterate_over_each_element_producing_a_list_showing_indexes_of_any_2s_5 [] {
  let result = ([1 2 3] | enumerate | par-each { |e| if $e.item == 2 { $"found 2 at ($e.index)!"} })
  assert ($result == [found 2 at 1!])
}


