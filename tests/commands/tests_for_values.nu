use std assert


# This is the custom command 1 for values:

#[test]
def values_get_the_values_from_the_record_produce_a_list_1 [] {
  let result = ({ mode:normal userid:31415 } | values)
  assert ($result == [normal, 31415])
}

# This is the custom command 2 for values:

#[test]
def values_values_are_ordered_by_the_column_order_of_the_record_2 [] {
  let result = ({ f:250 g:191 c:128 d:1024 e:2000 a:16 b:32 } | values)
  assert ($result == [250, 191, 128, 1024, 2000, 16, 32])
}

# This is the custom command 3 for values:

#[test]
def values_get_the_values_from_the_table_produce_a_list_of_lists_3 [] {
  let result = ([[name meaning]; [ls list] [mv move] [cd 'change directory']] | values)
  assert ($result == [[ls, mv, cd], [list, move, change directory]])
}


