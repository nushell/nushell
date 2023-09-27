use std assert

# Parameter name:
# sig type   : any
# name       : filename
# type       : positional
# shape      : string
# description: full path to file name to inspect


# This is the custom command 1 for file:

#[test]
def file_get_format_information_from_file_1 [] {
  let result = (file some.jpg)
  assert ($result == {description: Image, format: jpg, magic_offset: 0, magic_length: 2, magic_bytes: [FF, D8]})
}


