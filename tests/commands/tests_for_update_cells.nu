use std assert

# Parameter name:
# sig type   : table
# name       : closure
# type       : positional
# shape      : closure(any)
# description: the closure to run an update for each cell

# Parameter name:
# sig type   : table
# name       : columns
# type       : named
# shape      : table
# description: list of columns to update


# This is the custom command 1 for update_cells:

#[test]
def update_cells_update_the_zero_value_cells_to_empty_strings_1 [] {
  let result = ([
        ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
        [          37,            0,            0,            0,           37,            0,            0]
    ] | update cells { |value|
          if $value == 0 {
            ""
          } else {
            $value
          }
    })
  assert ($result == [{2021-04-16: 37, 2021-06-10: , 2021-09-18: , 2021-10-15: , 2021-11-16: 37, 2021-11-17: , 2021-11-18: }])
}

# This is the custom command 2 for update_cells:

#[test]
def update_cells_update_the_zero_value_cells_to_empty_strings_in_2_last_columns_2 [] {
  let result = ([
        ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
        [          37,            0,            0,            0,           37,            0,            0]
    ] | update cells -c ["2021-11-18", "2021-11-17"] { |value|
            if $value == 0 {
              ""
            } else {
              $value
            }
    })
  assert ($result == [{2021-04-16: 37, 2021-06-10: 0, 2021-09-18: 0, 2021-10-15: 0, 2021-11-16: 37, 2021-11-17: , 2021-11-18: }])
}


