use std assert

# Parameter name:
# sig type   : record
# name       : noheaders
# type       : switch
# shape      : 
# description: do not output the column names as the first row

# Parameter name:
# sig type   : table
# name       : noheaders
# type       : switch
# shape      : 
# description: do not output the column names as the first row


# This is the custom command 1 for to_tsv:

#[test]
def to_tsv_outputs_an_tsv_string_representing_the_contents_of_this_table_1 [] {
  let result = ([[foo bar]; [1 2]] | to tsv)
  assert ($result == foo	bar
1	2
)
}

# This is the custom command 2 for to_tsv:

#[test]
def to_tsv_outputs_an_tsv_string_representing_the_contents_of_this_record_2 [] {
  let result = ({a: 1 b: 2} | to tsv)
  assert ($result == a	b
1	2
)
}


