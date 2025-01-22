# Test std/formats when importing `use std *`
use std *

def test_data_multiline [--nuon] {
  use std *
  let lines = if $nuon {
    [
      "{a: 1}",
      "{a: 2}",
      "{a: 3}",
      "{a: 4}",
      "{a: 5}",
      "{a: 6}",
    ]
  } else {
    [
      "{\"a\":1}",
      "{\"a\":2}",
      "{\"a\":3}",
      "{\"a\":4}",
      "{\"a\":5}",
      "{\"a\":6}",
    ]
  }

  if $nu.os-info.name == "windows" {
    $lines | str join "\r\n"
  } else {
    $lines | str join "\n"
  }
}

#[test]
def from_ndjson_multiple_objects [] {
  let result = test_data_multiline | formats from ndjson
  let expect = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}]
  assert equal $result $expect "could not convert from NDJSON"
}

#[test]
def from_ndjson_single_object [] {
  let result = '{"a": 1}' | formats from ndjson
  let expect = [{a:1}]
  assert equal $result $expect "could not convert from NDJSON"
}

#[test]
def from_ndjson_invalid_object [] {
  assert error { '{"a":1' | formats from ndjson }
}

#[test]
def from_jsonl_multiple_objects [] {
  let result = test_data_multiline | formats from jsonl
  let expect = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}]
  assert equal $result $expect "could not convert from JSONL"
}

#[test]
def from_jsonl_single_object [] {
  let result = '{"a": 1}' | formats from jsonl
  let expect = [{a:1}]
  assert equal $result $expect "could not convert from JSONL"
}

#[test]
def from_jsonl_invalid_object [] {
  assert error { '{"a":1' | formats from jsonl }
}

#[test]
def to_ndjson_multiple_objects [] {
  let result = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}] | formats to ndjson | str trim
  let expect = test_data_multiline
  assert equal $result $expect "could not convert to NDJSON"
}

#[test]
def to_ndjson_single_object [] {
  let result = [{a:1}] | formats to ndjson | str trim
  let expect = "{\"a\":1}"
  assert equal $result $expect "could not convert to NDJSON"
}

#[test]
def to_jsonl_multiple_objects [] {
  let result = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}] | formats to jsonl | str trim
  let expect = test_data_multiline
  assert equal $result $expect "could not convert to JSONL"
}

#[test]
def to_jsonl_single_object [] {
  let result = [{a:1}] | formats to jsonl | str trim
  let expect = "{\"a\":1}"
  assert equal $result $expect "could not convert to JSONL"
}

#[test]
def from_ndnuon_multiple_objects [] {
  let result = test_data_multiline | formats from ndnuon
  let expect = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}]
  assert equal $result $expect "could not convert from NDNUON"
}

#[test]
def from_ndnuon_single_object [] {
  let result = '{a: 1}' | formats from ndnuon
  let expect = [{a:1}]
  assert equal $result $expect "could not convert from NDNUON"
}

#[test]
def from_ndnuon_invalid_object [] {
  assert error { '{"a":1' | formats from ndnuon }
}

#[test]
def to_ndnuon_multiple_objects [] {
  let result = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}] | formats to ndnuon | str trim
  let expect = test_data_multiline --nuon
  assert equal $result $expect "could not convert to NDNUON"
}

#[test]
def to_ndnuon_single_object [] {
  let result = [{a:1}] | formats to ndnuon | str trim
  let expect = "{a: 1}"
  assert equal $result $expect "could not convert to NDNUON"
}

#[test]
def to_ndnuon_multiline_strings [] {
  let result = "foo\n\\n\nbar" | formats to ndnuon
  let expect = '"foo\n\\n\nbar"'
  assert equal $result $expect "could not convert multiline string to NDNUON"
}

#[test]
def from_ndnuon_multiline_strings [] {
  let result = '"foo\n\\n\nbar"' | formats from ndnuon
  let expect = ["foo\n\\n\nbar"]
  assert equal $result $expect "could not convert multiline string from NDNUON"
}
