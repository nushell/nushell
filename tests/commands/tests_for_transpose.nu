use std assert

# Parameter name:
# sig type   : record
# name       : header-row
# type       : switch
# shape      : 
# description: treat the first row as column names

# Parameter name:
# sig type   : record
# name       : ignore-titles
# type       : switch
# shape      : 
# description: don't transpose the column names into values

# Parameter name:
# sig type   : record
# name       : as-record
# type       : switch
# shape      : 
# description: transfer to record if the result is a table and contains only one row

# Parameter name:
# sig type   : record
# name       : keep-last
# type       : switch
# shape      : 
# description: on repetition of record fields due to `header-row`, keep the last value obtained

# Parameter name:
# sig type   : record
# name       : keep-all
# type       : switch
# shape      : 
# description: on repetition of record fields due to `header-row`, keep all the values obtained

# Parameter name:
# sig type   : table
# name       : header-row
# type       : switch
# shape      : 
# description: treat the first row as column names

# Parameter name:
# sig type   : table
# name       : ignore-titles
# type       : switch
# shape      : 
# description: don't transpose the column names into values

# Parameter name:
# sig type   : table
# name       : as-record
# type       : switch
# shape      : 
# description: transfer to record if the result is a table and contains only one row

# Parameter name:
# sig type   : table
# name       : keep-last
# type       : switch
# shape      : 
# description: on repetition of record fields due to `header-row`, keep the last value obtained

# Parameter name:
# sig type   : table
# name       : keep-all
# type       : switch
# shape      : 
# description: on repetition of record fields due to `header-row`, keep all the values obtained


# This is the custom command 1 for transpose:

#[test]
def transpose_transposes_the_table_contents_with_default_column_names_1 [] {
  let result = ([[c1 c2]; [1 2]] | transpose)
  assert ($result == [{column0: c1, column1: 1}, {column0: c2, column1: 2}])
}

# This is the custom command 2 for transpose:

#[test]
def transpose_transposes_the_table_contents_with_specified_column_names_2 [] {
  let result = ([[c1 c2]; [1 2]] | transpose key val)
  assert ($result == [{key: c1, val: 1}, {key: c2, val: 2}])
}

# This is the custom command 3 for transpose:

#[test]
def transpose_transposes_the_table_without_column_names_and_specify_a_new_column_name_3 [] {
  let result = ([[c1 c2]; [1 2]] | transpose -i val)
  assert ($result == [{val: 1}, {val: 2}])
}

# This is the custom command 4 for transpose:

#[test]
def transpose_transfer_back_to_record_with__d_flag_4 [] {
  let result = ({c1: 1, c2: 2} | transpose | transpose -i -r -d)
  assert ($result == {c1: 1, c2: 2})
}


