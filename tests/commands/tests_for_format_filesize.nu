use std assert

# Parameter name:
# sig type   : filesize
# name       : format value
# type       : positional
# shape      : string
# description: the format into which convert the file sizes

# Parameter name:
# sig type   : record
# name       : format value
# type       : positional
# shape      : string
# description: the format into which convert the file sizes

# Parameter name:
# sig type   : table
# name       : format value
# type       : positional
# shape      : string
# description: the format into which convert the file sizes


# This is the custom command 1 for format_filesize:

#[test]
def format_filesize_convert_the_size_column_to_kb_1 [] {
  let result = (ls | format filesize KB size)
  assert ($result == )
}

# This is the custom command 2 for format_filesize:

#[test]
def format_filesize_convert_the_apparent_column_to_b_2 [] {
  let result = (du | format filesize B apparent)
  assert ($result == )
}

# This is the custom command 3 for format_filesize:

#[test]
def format_filesize_convert_the_size_data_to_mb_3 [] {
  let result = (4Gb | format filesize MB)
  assert ($result == 4000.0 MB)
}


