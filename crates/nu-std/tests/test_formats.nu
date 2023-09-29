use std assert

def ndjson_test_data1 [] {
  "{\"a\": 1}
{\"a\": 2}
{\"a\": 3}
{\"a\": 4}
{\"a\": 5}
{\"a\": 6}"
}

#[test]
def from_ndjson_multiple_objects [] {
  use std formats *
  let result = ndjson_test_data1 | from ndjson
  let expect = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}]
  assert equal $result $expect "could not convert from NDJSON"
}

#[test]
def from_ndjson_single_object [] {
  use std formats *
  let result = '{"a": 1}' | from ndjson
  let expect = [{a:1}]
  assert equal $result $expect "could not convert from NDJSON"
}

#[test]
def from_ndjson_invalid_object [] {
  use std formats *
  assert error { '{"a":1' | from ndjson }
}

#[test]
def from_jsonl_multiple_objects [] {
  use std formats *
  let result = ndjson_test_data1 | from jsonl
  let expect = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}]
  assert equal $result $expect "could not convert from JSONL"
}

#[test]
def from_jsonl_single_object [] {
  use std formats *
  let result = '{"a": 1}' | from jsonl
  let expect = [{a:1}]
  assert equal $result $expect "could not convert from JSONL"
}

#[test]
def from_jsonl_invalid_object [] {
  use std formats *
  assert error { '{"a":1' | from jsonl }
}

#[test]
def to_ndjson_multiple_objects [] {
  use std formats *
  let result = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}] | to ndjson | str trim
  let expect = ndjson_test_data1
  assert equal $result $expect "could not convert to NDJSON"
}

#[test]
def to_ndjson_single_object [] {
  use std formats *
  let result = [{a:1}] | to ndjson | str trim
  let expect = "{\"a\": 1}"
  assert equal $result $expect "could not convert to NDJSON"
}

#[test]
def to_jsonl_multiple_objects [] {
  use std formats *
  let result = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}] | to jsonl | str trim
  let expect = ndjson_test_data1
  assert equal $result $expect "could not convert to JSONL"
}

#[test]
def to_jsonl_single_object [] {
  use std formats *
  let result = [{a:1}] | to jsonl | str trim
  let expect = "{\"a\": 1}"
  assert equal $result $expect "could not convert to JSONL"
}
