use std assert

# Parameter name:
# sig type   : record
# name       : columns
# type       : rest
# shape      : string
# description: the columns to move

# Parameter name:
# sig type   : record
# name       : after
# type       : named
# shape      : string
# description: the column that will precede the columns moved

# Parameter name:
# sig type   : record
# name       : before
# type       : named
# shape      : string
# description: the column that will be the next after the columns moved

# Parameter name:
# sig type   : table
# name       : columns
# type       : rest
# shape      : string
# description: the columns to move

# Parameter name:
# sig type   : table
# name       : after
# type       : named
# shape      : string
# description: the column that will precede the columns moved

# Parameter name:
# sig type   : table
# name       : before
# type       : named
# shape      : string
# description: the column that will be the next after the columns moved


# This is the custom command 1 for move:

#[test]
def move_move_a_column_before_the_first_column_1 [] {
  let result = ([[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move index --before name)
  assert ($result == [{index: 1, name: foo, value: a}, {index: 2, name: bar, value: b}, {index: 3, name: baz, value: c}])
}

# This is the custom command 2 for move:

#[test]
def move_move_multiple_columns_after_the_last_column_and_reorder_them_2 [] {
  let result = ([[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move value name --after index)
  assert ($result == [{index: 1, value: a, name: foo}, {index: 2, value: b, name: bar}, {index: 3, value: c, name: baz}])
}

# This is the custom command 3 for move:

#[test]
def move_move_columns_of_a_record_3 [] {
  let result = ({ name: foo, value: a, index: 1 } | move name --before index)
  assert ($result == {value: a, name: foo, index: 1})
}


