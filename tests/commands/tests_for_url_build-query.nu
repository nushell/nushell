use std assert


# This is the custom command 1 for url_build-query:

#[test]
def url_build-query_outputs_a_query_string_representing_the_contents_of_this_record_1 [] {
  let result = ({ mode:normal userid:31415 } | url build-query)
  assert ($result == mode=normal&userid=31415)
}

# This is the custom command 2 for url_build-query:

#[test]
def url_build-query_outputs_a_query_string_representing_the_contents_of_this_1_row_table_2 [] {
  let result = ([[foo bar]; ["1" "2"]] | url build-query)
  assert ($result == foo=1&bar=2)
}

# This is the custom command 3 for url_build-query:

#[test]
def url_build-query_outputs_a_query_string_representing_the_contents_of_this_record_3 [] {
  let result = ({a:"AT&T", b: "AT T"} | url build-query)
  assert ($result == a=AT%26T&b=AT+T)
}


