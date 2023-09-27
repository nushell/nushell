use std assert

# Parameter name:
# sig type   : list<any>
# name       : prompt
# type       : positional
# shape      : string
# description: the prompt to display

# Parameter name:
# sig type   : list<any>
# name       : multi
# type       : switch
# shape      : 
# description: Use multiple results, you can press a to toggle all options on/off

# Parameter name:
# sig type   : list<any>
# name       : fuzzy
# type       : switch
# shape      : 
# description: Use a fuzzy select.


# This is the custom command 1 for input_list:

#[test]
def input_list_return_a_single_value_from_a_list_1 [] {
  let result = ([1 2 3 4 5] | input list 'Rate it')
  assert ($result == )
}

# This is the custom command 2 for input_list:

#[test]
def input_list_return_multiple_values_from_a_list_2 [] {
  let result = ([Banana Kiwi Pear Peach Strawberry] | input list -m 'Add fruits to the basket')
  assert ($result == )
}

# This is the custom command 3 for input_list:

#[test]
def input_list_return_a_single_record_from_a_table_with_fuzzy_search_3 [] {
  let result = (ls | input list -f 'Select the target')
  assert ($result == )
}


