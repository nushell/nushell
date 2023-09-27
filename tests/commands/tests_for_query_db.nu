use std assert

# Parameter name:
# sig type   : any
# name       : SQL
# type       : positional
# shape      : string
# description: SQL to execute against the database


# This is the custom command 1 for query_db:

#[test]
def query_db_execute_sql_against_a_sqlite_database_1 [] {
  let result = (open foo.db | query db "SELECT * FROM Bar")
  assert ($result == )
}


