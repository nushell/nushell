# Test std/formats when importing module-only
use std/assert
use std/formats *

def test_data_multiline [--nuon] {
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
  let result = test_data_multiline | from ndjson
  let expect = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}]
  assert equal $result $expect "could not convert from NDJSON"
}

#[test]
def from_ndjson_single_object [] {
  let result = '{"a": 1}' | from ndjson
  let expect = [{a:1}]
  assert equal $result $expect "could not convert from NDJSON"
}

#[test]
def from_ndjson_invalid_object [] {
  assert error { '{"a":1' | from ndjson }
}

#[test]
def from_jsonl_multiple_objects [] {
  let result = test_data_multiline | from jsonl
  let expect = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}]
  assert equal $result $expect "could not convert from JSONL"
}

#[test]
def from_jsonl_single_object [] {
  let result = '{"a": 1}' | from jsonl
  let expect = [{a:1}]
  assert equal $result $expect "could not convert from JSONL"
}

#[test]
def from_jsonl_invalid_object [] {
  assert error { '{"a":1' | from jsonl }
}

#[test]
def to_ndjson_multiple_objects [] {
  let result = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}] | to ndjson | str trim
  let expect = test_data_multiline
  assert equal $result $expect "could not convert to NDJSON"
}

#[test]
def to_ndjson_single_object [] {
  let result = [{a:1}] | to ndjson | str trim
  let expect = "{\"a\":1}"
  assert equal $result $expect "could not convert to NDJSON"
}

#[test]
def to_jsonl_multiple_objects [] {
  let result = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}] | to jsonl | str trim
  let expect = test_data_multiline
  assert equal $result $expect "could not convert to JSONL"
}

#[test]
def to_jsonl_single_object [] {
  let result = [{a:1}] | to jsonl | str trim
  let expect = "{\"a\":1}"
  assert equal $result $expect "could not convert to JSONL"
}

#[test]
def from_ndnuon_multiple_objects [] {
  let result = test_data_multiline | from ndnuon
  let expect = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}]
  assert equal $result $expect "could not convert from NDNUON"
}

#[test]
def from_ndnuon_single_object [] {
  let result = '{a: 1}' | from ndnuon
  let expect = [{a:1}]
  assert equal $result $expect "could not convert from NDNUON"
}

#[test]
def from_ndnuon_invalid_object [] {
  assert error { '{"a":1' | formats from ndnuon }
}

#[test]
def to_ndnuon_multiple_objects [] {
  let result = [{a:1},{a:2},{a:3},{a:4},{a:5},{a:6}] | to ndnuon | str trim
  let expect = test_data_multiline --nuon
  assert equal $result $expect "could not convert to NDNUON"
}

#[test]
def to_ndnuon_single_object [] {
  let result = [{a:1}] | to ndnuon | str trim
  let expect = "{a: 1}"
  assert equal $result $expect "could not convert to NDNUON"
}

#[test]
def to_ndnuon_multiline_strings [] {
  let result = "foo\n\\n\nbar" | to ndnuon
  let expect = '"foo\n\\n\nbar"'
  assert equal $result $expect "could not convert multiline string to NDNUON"
}

#[test]
def from_ndnuon_multiline_strings [] {
  let result = '"foo\n\\n\nbar"' | from ndnuon
  let expect = ["foo\n\\n\nbar"]
  assert equal $result $expect "could not convert multiline string from NDNUON"
}
