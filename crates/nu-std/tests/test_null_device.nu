use std *
use std/assert
use std/util [null-device, null_device]

#[test]
def assert_null_device_source [] {
    assert ((source $null_device) == null)
}

#[test]
def assert_null_device_open_var [] {
    assert ((open $null_device) == null)
}

#[test]
def assert_null_device_open_def [] {
    assert ((open (null-device)) == null)
}
