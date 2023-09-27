use std assert

# Parameter name:
# sig type   : any
# name       : query
# type       : positional
# shape      : string
# description: json path query


# This is the custom command 1 for json_path:

#[test]
def json_path_list_the_authors_of_all_books_in_the_store_1 [] {
  let result = (open -r test.json | json path '$.store.book[*].author')
  assert ($result == )
}


