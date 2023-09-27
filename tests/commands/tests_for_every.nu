use std assert

# Parameter name:
# sig type   : list<any>
# name       : stride
# type       : positional
# shape      : int
# description: how many rows to skip between (and including) each row returned

# Parameter name:
# sig type   : list<any>
# name       : skip
# type       : switch
# shape      : 
# description: skip the rows that would be returned, instead of selecting them


# This is the custom command 1 for every:

#[test]
def every_get_every_second_row_1 [] {
  let result = ([1 2 3 4 5] | every 2)
  assert ($result == [1, 3, 5])
}

# This is the custom command 2 for every:

#[test]
def every_skip_every_second_row_2 [] {
  let result = ([1 2 3 4 5] | every 2 --skip)
  assert ($result == [2, 4])
}


