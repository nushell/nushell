use std assert

# Parameter name:
# sig type   : list<string>
# name       : min-word-length
# type       : named
# shape      : int
# description: The minimum word length

# Parameter name:
# sig type   : list<string>
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: measure word length in grapheme clusters (requires -l)

# Parameter name:
# sig type   : list<string>
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: measure word length in UTF-8 bytes (default; requires -l; non-ASCII chars are length 2+)

# Parameter name:
# sig type   : string
# name       : min-word-length
# type       : named
# shape      : int
# description: The minimum word length

# Parameter name:
# sig type   : string
# name       : grapheme-clusters
# type       : switch
# shape      : 
# description: measure word length in grapheme clusters (requires -l)

# Parameter name:
# sig type   : string
# name       : utf-8-bytes
# type       : switch
# shape      : 
# description: measure word length in UTF-8 bytes (default; requires -l; non-ASCII chars are length 2+)


# This is the custom command 1 for split_words:

#[test]
def split_words_split_the_strings_words_into_separate_rows_1 [] {
  let result = ('hello world' | split words)
  assert ($result == [hello, world])
}

# This is the custom command 2 for split_words:

#[test]
def split_words_split_the_strings_words_of_at_least_3_characters_into_separate_rows_2 [] {
  let result = ('hello to the world' | split words -l 3)
  assert ($result == [hello, the, world])
}

# This is the custom command 3 for split_words:

#[test]
def split_words_a_real_world_example_of_splitting_words_3 [] {
  let result = (http get https://www.gutenberg.org/files/11/11-0.txt | str downcase | split words -l 2 | uniq -c | sort-by count --reverse | first 10)
  assert ($result == )
}


