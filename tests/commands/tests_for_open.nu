use std assert

# Parameter name:
# sig type   : nothing
# name       : filename
# type       : positional
# shape      : path
# description: the filename to use

# Parameter name:
# sig type   : nothing
# name       : filenames
# type       : rest
# shape      : path
# description: optional additional files to open

# Parameter name:
# sig type   : nothing
# name       : raw
# type       : switch
# shape      : 
# description: open file as raw binary

# Parameter name:
# sig type   : string
# name       : filename
# type       : positional
# shape      : path
# description: the filename to use

# Parameter name:
# sig type   : string
# name       : filenames
# type       : rest
# shape      : path
# description: optional additional files to open

# Parameter name:
# sig type   : string
# name       : raw
# type       : switch
# shape      : 
# description: open file as raw binary


# This is the custom command 1 for open:

#[test]
def open_open_a_file_with_structure_based_on_file_extension_or_sqlite_database_header_1 [] {
  let result = (open myfile.json)
  assert ($result == )
}

# This is the custom command 2 for open:

#[test]
def open_open_a_file_as_raw_bytes_2 [] {
  let result = (open myfile.json --raw)
  assert ($result == )
}

# This is the custom command 3 for open:

#[test]
def open_open_a_file_using_the_input_to_get_filename_3 [] {
  let result = ('myfile.txt' | open)
  assert ($result == )
}

# This is the custom command 4 for open:

#[test]
def open_open_a_file_and_decode_it_by_the_specified_encoding_4 [] {
  let result = (open myfile.txt --raw | decode utf-8)
  assert ($result == )
}

# This is the custom command 5 for open:

#[test]
def open_create_a_custom_from_parser_to_open_newline_delimited_json_files_with_open_5 [] {
  let result = (def "from ndjson" [] { from json -o }; open myfile.ndjson)
  assert ($result == )
}


