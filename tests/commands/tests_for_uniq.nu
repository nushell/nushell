use std assert

# Parameter name:
# sig type   : list<any>
# name       : count
# type       : switch
# shape      : 
# description: Return a table containing the distinct input values together with their counts

# Parameter name:
# sig type   : list<any>
# name       : repeated
# type       : switch
# shape      : 
# description: Return the input values that occur more than once

# Parameter name:
# sig type   : list<any>
# name       : ignore-case
# type       : switch
# shape      : 
# description: Compare input values case-insensitively

# Parameter name:
# sig type   : list<any>
# name       : unique
# type       : switch
# shape      : 
# description: Return the input values that occur once only


# This is the custom command 1 for uniq:

#[test]
def uniq_return_the_distinct_values_of_a_listtable_remove_duplicates_so_that_each_value_occurs_once_only_1 [] {
  let result = ([2 3 3 4] | uniq)
  assert ($result == [2, 3, 4])
}

# This is the custom command 2 for uniq:

#[test]
def uniq_return_the_input_values_that_occur_more_than_once_2 [] {
  let result = ([1 2 2] | uniq -d)
  assert ($result == [2])
}

# This is the custom command 3 for uniq:

#[test]
def uniq_return_the_input_values_that_occur_once_only_3 [] {
  let result = ([1 2 2] | uniq -u)
  assert ($result == [1])
}

# This is the custom command 4 for uniq:

#[test]
def uniq_ignore_differences_in_case_when_comparing_input_values_4 [] {
  let result = (['hello' 'goodbye' 'Hello'] | uniq -i)
  assert ($result == [hello, goodbye])
}

# This is the custom command 5 for uniq:

#[test]
def uniq_return_a_table_containing_the_distinct_input_values_together_with_their_counts_5 [] {
  let result = ([1 2 2] | uniq -c)
  assert ($result == [{value: 1, count: 1}, {value: 2, count: 2}])
}


