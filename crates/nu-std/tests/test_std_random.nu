use std/assert
use std/testing *
use std/random

@test
def "random dice rejects negative sides" [] {
  assert error {
    random dice --sides (-2)
  } "--sides (-2) should not have been accepted"
}

@test
def "random dice rejects zero sides" [] {
  assert error {
    random dice --sides 0
  } "--sides 0 should not have been accepted"
}

@test
def "random dice rejects negative dice" [] {
  assert error {
    random dice --dice (-2)
  } "--dice (-2) should not have been accepted"
}

@test
def "random dice rejects zero dice" [] {
  assert error {
    random dice --dice 0
  } "--dice 0 should not have been accepted"
}

@test
def "random dice rejects one-sided dice" [] {
  assert error {
    random dice --sides 1
  } "--sides 1 should not have been accepted"
}
