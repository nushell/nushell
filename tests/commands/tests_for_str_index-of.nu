use std assert

# Parameter name:
# sig type   : list<string>
# name       : string
# type       : positional
# shape      : string
# description: the string to find in the input

# Parameter name:
# sig type   : list<string>
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count indexes using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : list<string>
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count indexes using UTF-8 bytes (default; non-ASCII chars have length 2+)

# Parameter name:
# sig type   : list<string>
# name       : range
# type       : named
# shape      : range
# description: optional start and/or end index

# Parameter name:
# sig type   : list<string>
# name       : end
# type       : switch
# shape      : 
# description: search from the end of the input

# Parameter name:
# sig type   : record
# name       : string
# type       : positional
# shape      : string
# description: the string to find in the input

# Parameter name:
# sig type   : record
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count indexes using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : record
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count indexes using UTF-8 bytes (default; non-ASCII chars have length 2+)

# Parameter name:
# sig type   : record
# name       : range
# type       : named
# shape      : range
# description: optional start and/or end index

# Parameter name:
# sig type   : record
# name       : end
# type       : switch
# shape      : 
# description: search from the end of the input

# Parameter name:
# sig type   : string
# name       : string
# type       : positional
# shape      : string
# description: the string to find in the input

# Parameter name:
# sig type   : string
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count indexes using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : string
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count indexes using UTF-8 bytes (default; non-ASCII chars have length 2+)

# Parameter name:
# sig type   : string
# name       : range
# type       : named
# shape      : range
# description: optional start and/or end index

# Parameter name:
# sig type   : string
# name       : end
# type       : switch
# shape      : 
# description: search from the end of the input

# Parameter name:
# sig type   : table
# name       : string
# type       : positional
# shape      : string
# description: the string to find in the input

# Parameter name:
# sig type   : table
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count indexes using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : table
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count indexes using UTF-8 bytes (default; non-ASCII chars have length 2+)

# Parameter name:
# sig type   : table
# name       : range
# type       : named
# shape      : range
# description: optional start and/or end index

# Parameter name:
# sig type   : table
# name       : end
# type       : switch
# shape      : 
# description: search from the end of the input


# This is the custom command 1 for str_index-of:

#[test]
def str_index-of_returns_index_of_string_in_input_1 [] {
  let result = ( 'my_library.rb' | str index-of '.rb')
  assert ($result == 10)
}

# This is the custom command 2 for str_index-of:

#[test]
def str_index-of_count_length_using_grapheme_clusters_2 [] {
  let result = ('🇯🇵ほげ ふが ぴよ' | str index-of -g 'ふが')
  assert ($result == 4)
}

# This is the custom command 3 for str_index-of:

#[test]
def str_index-of_returns_index_of_string_in_input_within_arhs_open_range_3 [] {
  let result = ( '.rb.rb' | str index-of '.rb' -r 1..)
  assert ($result == 3)
}

# This is the custom command 4 for str_index-of:

#[test]
def str_index-of_returns_index_of_string_in_input_within_a_lhs_open_range_4 [] {
  let result = ( '123456' | str index-of '6' -r ..4)
  assert ($result == -1)
}

# This is the custom command 5 for str_index-of:

#[test]
def str_index-of_returns_index_of_string_in_input_within_a_range_5 [] {
  let result = ( '123456' | str index-of '3' -r 1..4)
  assert ($result == 2)
}

# This is the custom command 6 for str_index-of:

#[test]
def str_index-of_returns_index_of_string_in_input_6 [] {
  let result = ( '/this/is/some/path/file.txt' | str index-of '/' -e)
  assert ($result == 18)
}


