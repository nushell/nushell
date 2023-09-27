use std assert

# Parameter name:
# sig type   : list<string>
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: split on grapheme clusters

# Parameter name:
# sig type   : list<string>
# name       : code-points
# type       : switch
# shape      : 
# description: split on code points (default; splits combined characters)

# Parameter name:
# sig type   : string
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: split on grapheme clusters

# Parameter name:
# sig type   : string
# name       : code-points
# type       : switch
# shape      : 
# description: split on code points (default; splits combined characters)


# This is the custom command 1 for split_chars:

#[test]
def split_chars_split_the_string_into_a_list_of_characters_1 [] {
  let result = ('hello' | split chars)
  assert ($result == [h, e, l, l, o])
}

# This is the custom command 2 for split_chars:

#[test]
def split_chars_split_on_grapheme_clusters_2 [] {
  let result = ('🇯🇵ほげ' | split chars -g)
  assert ($result == [🇯🇵, ほ, げ])
}

# This is the custom command 3 for split_chars:

#[test]
def split_chars_split_multiple_strings_into_lists_of_characters_3 [] {
  let result = (['hello', 'world'] | split chars)
  assert ($result == [[h, e, l, l, o], [w, o, r, l, d]])
}


