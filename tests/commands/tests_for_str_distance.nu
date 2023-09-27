use std assert

# Parameter name:
# sig type   : record
# name       : compare-string
# type       : positional
# shape      : string
# description: the first string to compare

# Parameter name:
# sig type   : string
# name       : compare-string
# type       : positional
# shape      : string
# description: the first string to compare

# Parameter name:
# sig type   : table
# name       : compare-string
# type       : positional
# shape      : string
# description: the first string to compare


# This is the custom command 1 for str_distance:

#[test]
def str_distance_get_the_edit_distance_between_two_strings_1 [] {
  let result = ('nushell' | str distance 'nutshell')
  assert ($result == 1)
}

# This is the custom command 2 for str_distance:

#[test]
def str_distance_compute_edit_distance_between_strings_in_table_and_another_string_using_cell_paths_2 [] {
  let result = ([{a: 'nutshell' b: 'numetal'}] | str distance 'nushell' 'a' 'b')
  assert ($result == [{a: 1, b: 4}])
}

# This is the custom command 3 for str_distance:

#[test]
def str_distance_compute_edit_distance_between_strings_in_record_and_another_string_using_cell_paths_3 [] {
  let result = ({a: 'nutshell' b: 'numetal'} | str distance 'nushell' a b)
  assert ($result == {a: 1, b: 4})
}


