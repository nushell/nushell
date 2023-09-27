use std assert

# Parameter name:
# sig type   : list<string>
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count length using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : list<string>
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count length using UTF-8 bytes (default; all non-ASCII chars have length 2+)

# Parameter name:
# sig type   : record
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count length using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : record
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count length using UTF-8 bytes (default; all non-ASCII chars have length 2+)

# Parameter name:
# sig type   : string
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count length using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : string
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count length using UTF-8 bytes (default; all non-ASCII chars have length 2+)

# Parameter name:
# sig type   : table
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: count length using grapheme clusters (all visible chars have length 1)

# Parameter name:
# sig type   : table
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: count length using UTF-8 bytes (default; all non-ASCII chars have length 2+)


# This is the custom command 1 for str_length:

#[test]
def str_length_return_the_lengths_of_a_string_1 [] {
  let result = ('hello' | str length)
  assert ($result == 5)
}

# This is the custom command 2 for str_length:

#[test]
def str_length_count_length_using_grapheme_clusters_2 [] {
  let result = ('🇯🇵ほげ ふが ぴよ' | str length -g)
  assert ($result == 9)
}

# This is the custom command 3 for str_length:

#[test]
def str_length_return_the_lengths_of_multiple_strings_3 [] {
  let result = (['hi' 'there'] | str length)
  assert ($result == [2, 5])
}


