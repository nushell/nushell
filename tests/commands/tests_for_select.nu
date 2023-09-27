use std assert

# Parameter name:
# sig type   : list<any>
# name       : ignore-errors
# type       : switch
# shape      : 
# description: ignore missing data (make all cell path members optional)

# Parameter name:
# sig type   : record
# name       : ignore-errors
# type       : switch
# shape      : 
# description: ignore missing data (make all cell path members optional)

# Parameter name:
# sig type   : table
# name       : ignore-errors
# type       : switch
# shape      : 
# description: ignore missing data (make all cell path members optional)


# This is the custom command 1 for select:

#[test]
def select_select_a_column_in_a_table_1 [] {
  let result = ([{a: a b: b}] | select a)
  assert ($result == [{a: a}])
}

# This is the custom command 2 for select:

#[test]
def select_select_a_field_in_a_record_2 [] {
  let result = ({a: a b: b} | select a)
  assert ($result == {a: a})
}

# This is the custom command 3 for select:

#[test]
def select_select_just_the_name_column_3 [] {
  let result = (ls | select name)
  assert ($result == )
}

# This is the custom command 4 for select:

#[test]
def select_select_the_first_four_rows_this_is_the_same_as_first_4_4 [] {
  let result = (ls | select 0 1 2 3)
  assert ($result == )
}

# This is the custom command 5 for select:

#[test]
def select_select_columns_by_a_provided_list_of_columns_5 [] {
  let result = (let cols = [name type];[[name type size]; [Cargo.toml toml 1kb] [Cargo.lock toml 2kb]] | select $cols)
  assert ($result == )
}

# This is the custom command 6 for select:

#[test]
def select_select_rows_by_a_provided_list_of_rows_6 [] {
  let result = (let rows = [0 2];[[name type size]; [Cargo.toml toml 1kb] [Cargo.lock toml 2kb] [file.json json 3kb]] | select $rows)
  assert ($result == )
}


