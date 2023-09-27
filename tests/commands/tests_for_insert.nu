use std assert

# Parameter name:
# sig type   : list<any>
# name       : field
# type       : positional
# shape      : cell-path
# description: the name of the column to insert

# Parameter name:
# sig type   : list<any>
# name       : new value
# type       : positional
# shape      : any
# description: the new value to give the cell(s)

# Parameter name:
# sig type   : record
# name       : field
# type       : positional
# shape      : cell-path
# description: the name of the column to insert

# Parameter name:
# sig type   : record
# name       : new value
# type       : positional
# shape      : any
# description: the new value to give the cell(s)

# Parameter name:
# sig type   : table
# name       : field
# type       : positional
# shape      : cell-path
# description: the name of the column to insert

# Parameter name:
# sig type   : table
# name       : new value
# type       : positional
# shape      : any
# description: the new value to give the cell(s)


# This is the custom command 1 for insert:

#[test]
def insert_insert_a_new_entry_into_a_single_record_1 [] {
  let result = ({'name': 'nu', 'stars': 5} | insert alias 'Nushell')
  assert ($result == {name: nu, stars: 5, alias: Nushell})
}

# This is the custom command 2 for insert:

#[test]
def insert_insert_a_new_column_into_a_table_populating_all_rows_2 [] {
  let result = ([[project, lang]; ['Nushell', 'Rust']] | insert type 'shell')
  assert ($result == [{project: Nushell, lang: Rust, type: shell}])
}

# This is the custom command 3 for insert:

#[test]
def insert_insert_a_column_with_values_equal_to_their_row_index_plus_the_value_of_foo_in_each_row_3 [] {
  let result = ([[foo]; [7] [8] [9]] | enumerate | insert bar {|e| $e.item.foo + $e.index } | flatten)
  assert ($result == [{index: 0, foo: 7, bar: 7}, {index: 1, foo: 8, bar: 9}, {index: 2, foo: 9, bar: 11}])
}


