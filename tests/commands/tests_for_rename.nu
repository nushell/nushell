use std assert

# Parameter name:
# sig type   : record
# name       : column
# type       : named
# shape      : list<string>
# description: column name to be changed

# Parameter name:
# sig type   : record
# name       : block
# type       : named
# shape      : closure(any)
# description: A closure to apply changes on each column

# Parameter name:
# sig type   : table
# name       : column
# type       : named
# shape      : list<string>
# description: column name to be changed

# Parameter name:
# sig type   : table
# name       : block
# type       : named
# shape      : closure(any)
# description: A closure to apply changes on each column


# This is the custom command 1 for rename:

#[test]
def rename_rename_a_column_1 [] {
  let result = ([[a, b]; [1, 2]] | rename my_column)
  assert ($result == [{my_column: 1, b: 2}])
}

# This is the custom command 2 for rename:

#[test]
def rename_rename_many_columns_2 [] {
  let result = ([[a, b, c]; [1, 2, 3]] | rename eggs ham bacon)
  assert ($result == [{eggs: 1, ham: 2, bacon: 3}])
}

# This is the custom command 3 for rename:

#[test]
def rename_rename_a_specific_column_3 [] {
  let result = ([[a, b, c]; [1, 2, 3]] | rename -c [a ham])
  assert ($result == [{ham: 1, b: 2, c: 3}])
}

# This is the custom command 4 for rename:

#[test]
def rename_rename_the_fields_of_a_record_4 [] {
  let result = ({a: 1 b: 2} | rename x y)
  assert ($result == {x: 1, y: 2})
}

# This is the custom command 5 for rename:

#[test]
def rename_rename_fields_based_on_a_given_closure_5 [] {
  let result = ({abc: 1, bbc: 2} | rename -b {str replace -a 'b' 'z'})
  assert ($result == {azc: 1, zzc: 2})
}


