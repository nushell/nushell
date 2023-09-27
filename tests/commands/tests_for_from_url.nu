use std assert


# This is the custom command 1 for from_url:

#[test]
def from_url_convert_url_encoded_string_into_a_record_1 [] {
  let result = ('bread=baguette&cheese=comt%C3%A9&meat=ham&fat=butter' | from url)
  assert ($result == {bread: baguette, cheese: comté, meat: ham, fat: butter})
}


