use std assert

# Parameter name:
# sig type   : any
# name       : name
# type       : positional
# shape      : string
# description: the name of the column

# Parameter name:
# sig type   : list<any>
# name       : name
# type       : positional
# shape      : string
# description: the name of the column

# Parameter name:
# sig type   : range
# name       : name
# type       : positional
# shape      : string
# description: the name of the column


# This is the custom command 1 for wrap:

#[test]
def wrap_wrap_a_list_into_a_table_with_a_given_column_name_1 [] {
  let result = ([1 2 3] | wrap num)
  assert ($result == [{num: 1}, {num: 2}, {num: 3}])
}

# This is the custom command 2 for wrap:

#[test]
def wrap_wrap_a_range_into_a_table_with_a_given_column_name_2 [] {
  let result = (1..3 | wrap num)
  assert ($result == [{num: 1}, {num: 2}, {num: 3}])
}


