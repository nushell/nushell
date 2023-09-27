use std assert


# This is the custom command 1 for columns:

#[test]
def columns_get_the_columns_from_the_record_1 [] {
  let result = ({ acronym:PWD, meaning:'Print Working Directory' } | columns)
  assert ($result == [acronym, meaning])
}

# This is the custom command 2 for columns:

#[test]
def columns_get_the_columns_from_the_table_2 [] {
  let result = ([[name,age,grade]; [bill,20,a]] | columns)
  assert ($result == [name, age, grade])
}

# This is the custom command 3 for columns:

#[test]
def columns_get_the_first_column_from_the_table_3 [] {
  let result = ([[name,age,grade]; [bill,20,a]] | columns | first)
  assert ($result == )
}

# This is the custom command 4 for columns:

#[test]
def columns_get_the_second_column_from_the_table_4 [] {
  let result = ([[name,age,grade]; [bill,20,a]] | columns | select 1)
  assert ($result == )
}


