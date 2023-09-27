use std assert

# Parameter name:
# sig type   : list<any>
# name       : row_condition
# type       : positional
# shape      : condition
# description: Filter condition

# Parameter name:
# sig type   : range
# name       : row_condition
# type       : positional
# shape      : condition
# description: Filter condition

# Parameter name:
# sig type   : table
# name       : row_condition
# type       : positional
# shape      : condition
# description: Filter condition


# This is the custom command 1 for where:

#[test]
def where_filter_rows_of_a_table_according_to_a_condition_1 [] {
  let result = ([{a: 1} {a: 2}] | where a > 1)
  assert ($result == [{a: 2}])
}

# This is the custom command 2 for where:

#[test]
def where_filter_items_of_a_list_according_to_a_condition_2 [] {
  let result = ([1 2] | where {|x| $x > 1})
  assert ($result == [2])
}

# This is the custom command 3 for where:

#[test]
def where_list_all_files_in_the_current_directory_with_sizes_greater_than_2kb_3 [] {
  let result = (ls | where size > 2kb)
  assert ($result == )
}

# This is the custom command 4 for where:

#[test]
def where_list_only_the_files_in_the_current_directory_4 [] {
  let result = (ls | where type == file)
  assert ($result == )
}

# This is the custom command 5 for where:

#[test]
def where_list_all_files_with_names_that_contain_car_5 [] {
  let result = (ls | where name =~ "Car")
  assert ($result == )
}

# This is the custom command 6 for where:

#[test]
def where_list_all_files_that_were_modified_in_the_last_two_weeks_6 [] {
  let result = (ls | where modified >= (date now) - 2wk)
  assert ($result == )
}

# This is the custom command 7 for where:

#[test]
def where_find_files_whose_filenames_dont_begin_with_the_correct_sequential_number_7 [] {
  let result = (ls | where type == file | sort-by name -n | enumerate | where {|e| $e.item.name !~ $'^($e.index + 1)' } | each {|| get item })
  assert ($result == )
}

# This is the custom command 8 for where:

#[test]
def where_find_case_insensitively_files_called_readme_without_an_explicit_closure_8 [] {
  let result = (ls | where ($it.name | str downcase) =~ readme)
  assert ($result == )
}

# This is the custom command 9 for where:

#[test]
def where_same_as_above_but_with_regex_only_9 [] {
  let result = (ls | where name =~ '(?i)readme')
  assert ($result == )
}


