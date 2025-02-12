use std/assert
use std/testing *
use std-rfc/conversions *

@test
def range-into-list [] {
  assert equal (
    1..10 | into list
  ) (
    [ 1 2 3 4 5 6 7 8 9 10 ]
  )
}

@test
def string-into-list [] {
  assert equal (
    "foo" | into list
  ) (
    [ foo ]
  )
}

@test
def range-stride-into-list [] {
  assert equal (
    0..2..10 | into list
  ) (
    [ 0 2 4 6 8 10 ]
  )
}

@test
def null-into-list [] {
  assert equal (
    null | into list | get 0 | describe
  ) (
    "nothing"
  )
}

@test
def list-into-list [] {
  assert equal (
    [ foo bar baz ] | into list
  ) (
    [ foo bar baz ]
  )

}

@test
def table-into-columns--roundtrip [] {
  assert equal (
    ls
  ) (
    ls | table-into-columns | columns-into-table
  )
}

const test_record_of_lists = {
  a: [ 1 2 3 ]
  b: [ 4 5 6 ]
}

@test
def record-into-columns--simple [] {
  let actual = (
    $test_record_of_lists
    | record-into-columns
    | get 1.b.2
  )

  let expected = 6

  assert equal $actual $expected
}

@test
def table-into-columns--simple [] {
  let actual = (
    ls | table-into-columns | get 1 | columns | get 0
  )
  let expected = 'type'

  assert equal $actual $expected
}

@test
def name-values--simple [] {
  let actual = (
    [ 1 2 3 ] | name-values one two three
    | get 'two'
  )

  let expected = 2

  assert equal $actual $expected
}

@test
def name-values--missing-keyname [] {
  let actual = (
    [ 1 2 3 ] | name-values one two
    | columns
  )

  # Column/key names are strings, even those that came from the index ('2')
  let expected = [ 'one' 'two' '2' ]

  assert equal $actual $expected
}
