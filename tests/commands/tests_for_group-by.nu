use std assert

# Parameter name:
# sig type   : list<any>
# name       : grouper
# type       : positional
# shape      : one_of(cell-path, block, closure(), closure(any))
# description: the path to the column to group on


# This is the custom command 1 for group-by:

#[test]
def group-by_group_items_by_the_type_columns_values_1 [] {
  let result = (ls | group-by type)
  assert ($result == )
}

# This is the custom command 2 for group-by:

#[test]
def group-by_group_items_by_the_foo_columns_values_ignoring_records_without_a_foo_column_2 [] {
  let result = (open cool.json | group-by foo?)
  assert ($result == )
}

# This is the custom command 3 for group-by:

#[test]
def group-by_group_using_a_block_which_is_evaluated_against_each_input_value_3 [] {
  let result = ([foo.txt bar.csv baz.txt] | group-by { path parse | get extension })
  assert ($result == {txt: [foo.txt, baz.txt], csv: [bar.csv]})
}

# This is the custom command 4 for group-by:

#[test]
def group-by_you_can_also_group_by_raw_values_by_leaving_out_the_argument_4 [] {
  let result = (['1' '3' '1' '3' '2' '1' '1'] | group-by)
  assert ($result == {1: [1, 1, 1, 1], 3: [3, 3], 2: [2]})
}


