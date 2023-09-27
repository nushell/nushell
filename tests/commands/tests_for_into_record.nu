use std assert


# This is the custom command 1 for into_record:

#[test]
def into_record_convert_from_one_row_table_to_record_1 [] {
  let result = ([[value]; [false]] | into record)
  assert ($result == {value: false})
}

# This is the custom command 2 for into_record:

#[test]
def into_record_convert_from_list_to_record_2 [] {
  let result = ([1 2 3] | into record)
  assert ($result == {0: 1, 1: 2, 2: 3})
}

# This is the custom command 3 for into_record:

#[test]
def into_record_convert_from_range_to_record_3 [] {
  let result = (0..2 | into record)
  assert ($result == {0: 0, 1: 1, 2: 2})
}

# This is the custom command 4 for into_record:

#[test]
def into_record_convert_duration_to_record_weeks_max_4 [] {
  let result = ((-500day - 4hr - 5sec) | into record)
  assert ($result == {week: 71, day: 3, hour: 4, second: 5, sign: -})
}

# This is the custom command 5 for into_record:

#[test]
def into_record_convert_record_to_record_5 [] {
  let result = ({a: 1, b: 2} | into record)
  assert ($result == {a: 1, b: 2})
}

# This is the custom command 6 for into_record:

#[test]
def into_record_convert_date_to_record_6 [] {
  let result = (2020-04-12T22:10:57+02:00 | into record)
  assert ($result == {year: 2020, month: 4, day: 12, hour: 22, minute: 10, second: 57, timezone: +02:00})
}


