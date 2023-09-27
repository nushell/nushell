use std assert

# Parameter name:
# sig type   : string
# name       : separator
# type       : named
# shape      : string
# description: a character to separate columns (either single char or 4 byte unicode sequence), defaults to ','

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


# This is the custom command 1 for from_csv:

#[test]
def from_csv_convert_comma_separated_data_to_a_table_1 [] {
  let result = ("ColA,ColB
1,2" | from csv)
  assert ($result == [{ColA: 1, ColB: 2}])
}

# This is the custom command 2 for from_csv:

#[test]
def from_csv_convert_comma_separated_data_to_a_table_ignoring_headers_2 [] {
  let result = (open data.txt | from csv --noheaders)
  assert ($result == )
}

# This is the custom command 3 for from_csv:

#[test]
def from_csv_convert_semicolon_separated_data_to_a_table_3 [] {
  let result = (open data.txt | from csv --separator ';')
  assert ($result == )
}

# This is the custom command 4 for from_csv:

#[test]
def from_csv_convert_comma_separated_data_to_a_table_ignoring_lines_starting_with__4 [] {
  let result = (open data.txt | from csv --comment '#')
  assert ($result == )
}

# This is the custom command 5 for from_csv:

#[test]
def from_csv_convert_comma_separated_data_to_a_table_dropping_all_possible_whitespaces_around_header_names_and_field_values_5 [] {
  let result = (open data.txt | from csv --trim all)
  assert ($result == )
}

# This is the custom command 6 for from_csv:

#[test]
def from_csv_convert_comma_separated_data_to_a_table_dropping_all_possible_whitespaces_around_header_names_6 [] {
  let result = (open data.txt | from csv --trim headers)
  assert ($result == )
}

# This is the custom command 7 for from_csv:

#[test]
def from_csv_convert_comma_separated_data_to_a_table_dropping_all_possible_whitespaces_around_field_values_7 [] {
  let result = (open data.txt | from csv --trim fields)
  assert ($result == )
}


