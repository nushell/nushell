use std assert

# Parameter name:
# sig type   : any
# name       : file_name
# type       : positional
# shape      : string
# description: Specify the filename to save the database to

# Parameter name:
# sig type   : any
# name       : table_name
# type       : named
# shape      : string
# description: Specify table name to store the data in


# This is the custom command 1 for into_sqlite:

#[test]
def into_sqlite_convert_ls_entries_into_a_sqlite_database_with_main_as_the_table_name_1 [] {
  let result = (ls | into sqlite my_ls.db)
  assert ($result == )
}

# This is the custom command 2 for into_sqlite:

#[test]
def into_sqlite_convert_ls_entries_into_a_sqlite_database_with_my_table_as_the_table_name_2 [] {
  let result = (ls | into sqlite my_ls.db -t my_table)
  assert ($result == )
}

# This is the custom command 3 for into_sqlite:

#[test]
def into_sqlite_convert_table_literal_into_a_sqlite_database_with_main_as_the_table_name_3 [] {
  let result = ([[name]; [-----] [someone] [=====] [somename] ['(((((']] | into sqlite filename.db)
  assert ($result == )
}

# This is the custom command 4 for into_sqlite:

#[test]
def into_sqlite_convert_a_variety_of_values_in_table_literal_form_into_a_sqlite_database_4 [] {
  let result = ([one 2 5.2 six true 100mib 25sec] | into sqlite variety.db)
  assert ($result == )
}


