use std assert

# Parameter name:
# sig type   : string
# name       : comment
# type       : named
# shape      : string
# description: a comment character to ignore lines starting with it

# Parameter name:
# sig type   : string
# name       : quote
# type       : named
# shape      : string
# description: a quote character to ignore separators in strings, defaults to '"'

# Parameter name:
# sig type   : string
# name       : escape
# type       : named
# shape      : string
# description: an escape character for strings containing the quote character

# Parameter name:
# sig type   : string
# name       : noheaders
# type       : switch
# shape      : 
# description: don't treat the first row as column names

# Parameter name:
# sig type   : string
# name       : flexible
# type       : switch
# shape      : 
# description: allow the number of fields in records to be variable

# Parameter name:
# sig type   : string
# name       : no-infer
# type       : switch
# shape      : 
# description: no field type inferencing

# Parameter name:
# sig type   : string
# name       : trim
# type       : named
# shape      : string
# description: drop leading and trailing whitespaces around headers names and/or field values


# This is the custom command 1 for from_tsv:

#[test]
def from_tsv_convert_tab_separated_data_to_a_table_1 [] {
  let result = ("ColA	ColB
1	2" | from tsv)
  assert ($result == [{ColA: 1, ColB: 2}])
}

# This is the custom command 2 for from_tsv:

#[test]
def from_tsv_create_a_tsv_file_with_header_columns_and_open_it_2 [] {
  let result = ($'c1(char tab)c2(char tab)c3(char nl)1(char tab)2(char tab)3' | save tsv-data | open tsv-data | from tsv)
  assert ($result == )
}

# This is the custom command 3 for from_tsv:

#[test]
def from_tsv_create_a_tsv_file_without_header_columns_and_open_it_3 [] {
  let result = ($'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv -n)
  assert ($result == )
}

# This is the custom command 4 for from_tsv:

#[test]
def from_tsv_create_a_tsv_file_without_header_columns_and_open_it_removing_all_unnecessary_whitespaces_4 [] {
  let result = ($'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim all)
  assert ($result == )
}

# This is the custom command 5 for from_tsv:

#[test]
def from_tsv_create_a_tsv_file_without_header_columns_and_open_it_removing_all_unnecessary_whitespaces_in_the_header_names_5 [] {
  let result = ($'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim headers)
  assert ($result == )
}

# This is the custom command 6 for from_tsv:

#[test]
def from_tsv_create_a_tsv_file_without_header_columns_and_open_it_removing_all_unnecessary_whitespaces_in_the_field_values_6 [] {
  let result = ($'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim fields)
  assert ($result == )
}


