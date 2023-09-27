use std assert

# Parameter name:
# sig type   : string
# name       : encoding
# type       : positional
# shape      : string
# description: the text encoding to use

# Parameter name:
# sig type   : string
# name       : ignore-errors
# type       : switch
# shape      : 
# description: when a character isn't in the given encoding, replace with a HTML entity (like `&#127880;`)


# This is the custom command 1 for encode:

#[test]
def encode_encode_an_utf_8_string_into_shift_jis_1 [] {
  let result = ("負けると知って戦うのが、遥かに美しいのだ" | encode shift-jis)
  assert ($result == [149, 137, 130, 175, 130, 233, 130, 198, 146, 109, 130, 193, 130, 196, 144, 237, 130, 164, 130, 204, 130, 170, 129, 65, 151, 121, 130, 169, 130, 201, 148, 252, 130, 181, 130, 162, 130, 204, 130, 190])
}

# This is the custom command 2 for encode:

#[test]
def encode_replace_characters_with_html_entities_if_they_cant_be_encoded_2 [] {
  let result = ("🎈" | encode -i shift-jis)
  assert ($result == [38, 35, 49, 50, 55, 56, 56, 48, 59])
}


