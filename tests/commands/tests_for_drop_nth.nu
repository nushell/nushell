use std assert

# Parameter name:
# sig type   : list<any>
# name       : row number or row range
# type       : positional
# shape      : any
# description: the number of the row to drop or a range to drop consecutive rows


# This is the custom command 1 for drop_nth:

#[test]
def drop_nth_drop_the_first_second_and_third_row_1 [] {
  let result = ([sam,sarah,2,3,4,5] | drop nth 0 1 2)
  assert ($result == [3, 4, 5])
}

# This is the custom command 2 for drop_nth:

#[test]
def drop_nth_drop_the_first_second_and_third_row_2 [] {
  let result = ([0,1,2,3,4,5] | drop nth 0 1 2)
  assert ($result == [3, 4, 5])
}

# This is the custom command 3 for drop_nth:

#[test]
def drop_nth_drop_rows_0_2_4_3 [] {
  let result = ([0,1,2,3,4,5] | drop nth 0 2 4)
  assert ($result == [1, 3, 5])
}

# This is the custom command 4 for drop_nth:

#[test]
def drop_nth_drop_rows_2_0_4_4 [] {
  let result = ([0,1,2,3,4,5] | drop nth 2 0 4)
  assert ($result == [1, 3, 5])
}

# This is the custom command 5 for drop_nth:

#[test]
def drop_nth_drop_range_rows_from_second_to_fourth_5 [] {
  let result = ([first second third fourth fifth] | drop nth (1..3))
  assert ($result == [first, fifth])
}

# This is the custom command 6 for drop_nth:

#[test]
def drop_nth_drop_all_rows_except_first_row_6 [] {
  let result = ([0,1,2,3,4,5] | drop nth 1..)
  assert ($result == [0])
}

# This is the custom command 7 for drop_nth:

#[test]
def drop_nth_drop_rows_345_7 [] {
  let result = ([0,1,2,3,4,5] | drop nth 3..)
  assert ($result == [0, 1, 2])
}


