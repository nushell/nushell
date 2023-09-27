use std assert

# Parameter name:
# sig type   : list<any>
# name       : columns
# type       : rest
# shape      : any
# description: the columns to compact from the table

# Parameter name:
# sig type   : table
# name       : columns
# type       : rest
# shape      : any
# description: the columns to compact from the table


# This is the custom command 1 for compact:

#[test]
def compact_filter_out_all_records_where_hello_is_null_returns_nothing_1 [] {
  let result = ([["Hello" "World"]; [null 3]] | compact Hello)
  assert ($result == [])
}

# This is the custom command 2 for compact:

#[test]
def compact_filter_out_all_records_where_world_is_null_returns_the_table_2 [] {
  let result = ([["Hello" "World"]; [null 3]] | compact World)
  assert ($result == [{Hello: , World: 3}])
}

# This is the custom command 3 for compact:

#[test]
def compact_filter_out_all_instances_of_nothing_from_a_list_returns_12_3 [] {
  let result = ([1, null, 2] | compact)
  assert ($result == [1, 2])
}


