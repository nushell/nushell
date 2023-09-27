use std assert

# Parameter name:
# sig type   : list<any>
# name       : rows
# type       : positional
# shape      : range
# description: range of rows to return: Eg) 4..7 (=> from 4 to 7)


# This is the custom command 1 for range:

#[test]
def range_get_the_last_2_items_1 [] {
  let result = ([0,1,2,3,4,5] | range 4..5)
  assert ($result == [4, 5])
}

# This is the custom command 2 for range:

#[test]
def range_get_the_last_2_items_2 [] {
  let result = ([0,1,2,3,4,5] | range (-2)..)
  assert ($result == [4, 5])
}

# This is the custom command 3 for range:

#[test]
def range_get_the_next_to_last_2_items_3 [] {
  let result = ([0,1,2,3,4,5] | range (-3)..-2)
  assert ($result == [3, 4])
}


