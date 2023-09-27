use std assert

# Parameter name:
# sig type   : list<any>
# name       : n
# type       : positional
# shape      : int
# description: the number of elements to skip

# Parameter name:
# sig type   : table
# name       : n
# type       : positional
# shape      : int
# description: the number of elements to skip


# This is the custom command 1 for skip:

#[test]
def skip_skip_the_first_value_of_a_list_1 [] {
  let result = ([2 4 6 8] | skip 1)
  assert ($result == [4, 6, 8])
}

# This is the custom command 2 for skip:

#[test]
def skip_skip_two_rows_of_a_table_2 [] {
  let result = ([[editions]; [2015] [2018] [2021]] | skip 2)
  assert ($result == [{editions: 2021}])
}


