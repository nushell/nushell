use std assert

# Parameter name:
# sig type   : list<string>
# name       : range
# type       : positional
# shape      : any
# description: the indexes to substring [start end]

# Parameter name:
# sig type   : list<string>
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count indexes and split using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : list<string>
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count indexes and split using UTF-8 bytes (default; non-ASCII chars have length 2+)

# Parameter name:
# sig type   : record
# name       : range
# type       : positional
# shape      : any
# description: the indexes to substring [start end]

# Parameter name:
# sig type   : record
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count indexes and split using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : record
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count indexes and split using UTF-8 bytes (default; non-ASCII chars have length 2+)

# Parameter name:
# sig type   : string
# name       : range
# type       : positional
# shape      : any
# description: the indexes to substring [start end]

# Parameter name:
# sig type   : string
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count indexes and split using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : string
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count indexes and split using UTF-8 bytes (default; non-ASCII chars have length 2+)

# Parameter name:
# sig type   : table
# name       : range
# type       : positional
# shape      : any
# description: the indexes to substring [start end]

# Parameter name:
# sig type   : table
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count indexes and split using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : table
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count indexes and split using UTF-8 bytes (default; non-ASCII chars have length 2+)


# This is the custom command 1 for str_substring:

#[test]
def str_substring_get_a_substring_nushell_from_the_text_good_nushell_using_a_range_1 [] {
  let result = ( 'good nushell' | str substring 5..12)
  assert ($result == nushell)
}

# This is the custom command 2 for str_substring:

#[test]
def str_substring_count_indexes_and_split_using_grapheme_clusters_2 [] {
  let result = ( '🇯🇵ほげ ふが ぴよ' | str substring -g 4..6)
  assert ($result == ふが)
}


