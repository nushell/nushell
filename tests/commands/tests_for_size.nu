use std assert


# This is the custom command 1 for size:

#[test]
def size_count_the_number_of_words_in_a_string_1 [] {
  let result = ("There are seven words in this sentence" | size)
  assert ($result == {lines: 1, words: 7, bytes: 38, chars: 38, graphemes: 38})
}

# This is the custom command 2 for size:

#[test]
def size_counts_unicode_characters_2 [] {
  let result = ('今天天气真好' | size )
  assert ($result == {lines: 1, words: 6, bytes: 18, chars: 6, graphemes: 6})
}

# This is the custom command 3 for size:

#[test]
def size_counts_unicode_characters_correctly_in_a_string_3 [] {
  let result = ("Amélie Amelie" | size)
  assert ($result == {lines: 1, words: 2, bytes: 15, chars: 14, graphemes: 13})
}


