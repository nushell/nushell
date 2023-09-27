use std assert


# This is the custom command 1 for schema:

#[test]
def schema_show_the_schema_of_a_sqlite_database_1 [] {
  let result = (open foo.db | schema)
  assert ($result == )
}


