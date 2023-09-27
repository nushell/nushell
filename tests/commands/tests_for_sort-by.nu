use std assert

# Parameter name:
# sig type   : list<any>
# name       : columns
# type       : rest
# shape      : any
# description: the column(s) to sort by

# Parameter name:
# sig type   : list<any>
# name       : reverse
# type       : switch
# shape      : 
# description: Sort in reverse order

# Parameter name:
# sig type   : list<any>
# name       : ignore-case
# type       : switch
# shape      : 
# description: Sort string-based columns case-insensitively

# Parameter name:
# sig type   : list<any>
# name       : natural
# type       : switch
# shape      : 
# description: Sort alphanumeric string-based columns naturally (1, 9, 10, 99, 100, ...)

# Parameter name:
# sig type   : table
# name       : columns
# type       : rest
# shape      : any
# description: the column(s) to sort by

# Parameter name:
# sig type   : table
# name       : reverse
# type       : switch
# shape      : 
# description: Sort in reverse order

# Parameter name:
# sig type   : table
# name       : ignore-case
# type       : switch
# shape      : 
# description: Sort string-based columns case-insensitively

# Parameter name:
# sig type   : table
# name       : natural
# type       : switch
# shape      : 
# description: Sort alphanumeric string-based columns naturally (1, 9, 10, 99, 100, ...)


# This is the custom command 1 for sort-by:

#[test]
def sort-by_sort_files_by_modified_date_1 [] {
  let result = (ls | sort-by modified)
  assert ($result == )
}

# This is the custom command 2 for sort-by:

#[test]
def sort-by_sort_files_by_name_case_insensitive_2 [] {
  let result = (ls | sort-by name -i)
  assert ($result == )
}

# This is the custom command 3 for sort-by:

#[test]
def sort-by_sort_a_table_by_a_column_reversed_order_3 [] {
  let result = ([[fruit count]; [apple 9] [pear 3] [orange 7]] | sort-by fruit -r)
  assert ($result == [{fruit: pear, count: 3}, {fruit: orange, count: 7}, {fruit: apple, count: 9}])
}


