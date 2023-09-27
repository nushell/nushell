use std assert

# Parameter name:
# sig type   : list<any>
# name       : columns
# type       : rest
# shape      : any
# description: the column(s) to filter by

# Parameter name:
# sig type   : list<any>
# name       : count
# type       : switch
# shape      : 
# description: Return a table containing the distinct input values together with their counts

# Parameter name:
# sig type   : list<any>
# name       : repeated
# type       : switch
# shape      : 
# description: Return the input values that occur more than once

# Parameter name:
# sig type   : list<any>
# name       : ignore-case
# type       : switch
# shape      : 
# description: Ignore differences in case when comparing input values

# Parameter name:
# sig type   : list<any>
# name       : unique
# type       : switch
# shape      : 
# description: Return the input values that occur once only

# Parameter name:
# sig type   : table
# name       : columns
# type       : rest
# shape      : any
# description: the column(s) to filter by

# Parameter name:
# sig type   : table
# name       : count
# type       : switch
# shape      : 
# description: Return a table containing the distinct input values together with their counts

# Parameter name:
# sig type   : table
# name       : repeated
# type       : switch
# shape      : 
# description: Return the input values that occur more than once

# Parameter name:
# sig type   : table
# name       : ignore-case
# type       : switch
# shape      : 
# description: Ignore differences in case when comparing input values

# Parameter name:
# sig type   : table
# name       : unique
# type       : switch
# shape      : 
# description: Return the input values that occur once only


# This is the custom command 1 for uniq-by:

#[test]
def uniq-by_get_rows_from_table_filtered_by_column_uniqueness_1 [] {
  let result = ([[fruit count]; [apple 9] [apple 2] [pear 3] [orange 7]] | uniq-by fruit)
  assert ($result == [{fruit: apple, count: 9}, {fruit: pear, count: 3}, {fruit: orange, count: 7}])
}


