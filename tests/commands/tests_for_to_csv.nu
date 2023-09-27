use std assert

# Parameter name:
# sig type   : record
# name       : separator
# type       : named
# shape      : string
# description: a character to separate columns, defaults to ','

# Parameter name:
# sig type   : record
# name       : noheaders
# type       : switch
# shape      : 
# description: do not output the columns names as the first row

# Parameter name:
# sig type   : table
# name       : separator
# type       : named
# shape      : string
# description: a character to separate columns, defaults to ','

# Parameter name:
# sig type   : table
# name       : noheaders
# type       : switch
# shape      : 
# description: do not output the columns names as the first row


# This is the custom command 1 for to_csv:

#[test]
def to_csv_outputs_an_csv_string_representing_the_contents_of_this_table_1 [] {
  let result = ([[foo bar]; [1 2]] | to csv)
  assert ($result == foo,bar
1,2
)
}

# This is the custom command 2 for to_csv:

#[test]
def to_csv_outputs_an_csv_string_representing_the_contents_of_this_table_2 [] {
  let result = ([[foo bar]; [1 2]] | to csv -s ';' )
  assert ($result == foo;bar
1;2
)
}

# This is the custom command 3 for to_csv:

#[test]
def to_csv_outputs_an_csv_string_representing_the_contents_of_this_record_3 [] {
  let result = ({a: 1 b: 2} | to csv)
  assert ($result == a,b
1,2
)
}


