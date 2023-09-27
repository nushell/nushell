use std assert

# Parameter name:
# sig type   : list<string>
# name       : all
# type       : switch
# shape      : 
# description: encode all non-alphanumeric chars including `/`, `.`, `:`

# Parameter name:
# sig type   : record
# name       : all
# type       : switch
# shape      : 
# description: encode all non-alphanumeric chars including `/`, `.`, `:`

# Parameter name:
# sig type   : string
# name       : all
# type       : switch
# shape      : 
# description: encode all non-alphanumeric chars including `/`, `.`, `:`

# Parameter name:
# sig type   : table
# name       : all
# type       : switch
# shape      : 
# description: encode all non-alphanumeric chars including `/`, `.`, `:`


# This is the custom command 1 for url_encode:

#[test]
def url_encode_encode_a_url_with_escape_characters_1 [] {
  let result = ('https://example.com/foo bar' | url encode)
  assert ($result == https://example.com/foo%20bar)
}

# This is the custom command 2 for url_encode:

#[test]
def url_encode_encode_multiple_urls_with_escape_characters_in_list_2 [] {
  let result = (['https://example.com/foo bar' 'https://example.com/a>b' '中文字/eng/12 34'] | url encode)
  assert ($result == [https://example.com/foo%20bar, https://example.com/a%3Eb, %E4%B8%AD%E6%96%87%E5%AD%97/eng/12%2034])
}

# This is the custom command 3 for url_encode:

#[test]
def url_encode_encode_all_non_alphanumeric_chars_with_all_flag_3 [] {
  let result = ('https://example.com/foo bar' | url encode --all)
  assert ($result == https%3A%2F%2Fexample%2Ecom%2Ffoo%20bar)
}


