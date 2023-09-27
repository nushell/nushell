use std assert

# Parameter name:
# sig type   : list<any>
# name       : field
# type       : positional
# shape      : cell-path
# description: the name of the column to update or insert

# Parameter name:
# sig type   : list<any>
# name       : replacement value
# type       : positional
# shape      : any
# description: the new value to give the cell(s), or a closure to create the value

# Parameter name:
# sig type   : record
# name       : field
# type       : positional
# shape      : cell-path
# description: the name of the column to update or insert

# Parameter name:
# sig type   : record
# name       : replacement value
# type       : positional
# shape      : any
# description: the new value to give the cell(s), or a closure to create the value

# Parameter name:
# sig type   : table
# name       : field
# type       : positional
# shape      : cell-path
# description: the name of the column to update or insert

# Parameter name:
# sig type   : table
# name       : replacement value
# type       : positional
# shape      : any
# description: the new value to give the cell(s), or a closure to create the value


# This is the custom command 1 for upsert:

#[test]
def upsert_update_a_records_value_1 [] {
  let result = ({'name': 'nu', 'stars': 5} | upsert name 'Nushell')
  assert ($result == {name: Nushell, stars: 5})
}

# This is the custom command 2 for upsert:

#[test]
def upsert_update_each_row_of_a_table_2 [] {
  let result = ([[name lang]; [Nushell ''] [Reedline '']] | upsert lang 'Rust')
  assert ($result == [{name: Nushell, lang: Rust}, {name: Reedline, lang: Rust}])
}

# This is the custom command 3 for upsert:

#[test]
def upsert_insert_a_new_entry_into_a_single_record_3 [] {
  let result = ({'name': 'nu', 'stars': 5} | upsert language 'Rust')
  assert ($result == {name: nu, stars: 5, language: Rust})
}

# This is the custom command 4 for upsert:

#[test]
def upsert_use_in_closure_form_for_more_involved_updating_logic_4 [] {
  let result = ([[count fruit]; [1 'apple']] | enumerate | upsert item.count {|e| ($e.item.fruit | str length) + $e.index } | get item)
  assert ($result == [{count: 5, fruit: apple}])
}

# This is the custom command 5 for upsert:

#[test]
def upsert_upsert_an_int_into_a_list_updating_an_existing_value_based_on_the_index_5 [] {
  let result = ([1 2 3] | upsert 0 2)
  assert ($result == [2, 2, 3])
}

# This is the custom command 6 for upsert:

#[test]
def upsert_upsert_an_int_into_a_list_inserting_a_new_value_based_on_the_index_6 [] {
  let result = ([1 2 3] | upsert 3 4)
  assert ($result == [1, 2, 3, 4])
}


