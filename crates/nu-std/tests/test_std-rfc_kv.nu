const kv_module = if ("sqlite" in (version).features) { "std-rfc/kv" } else { null }
use $kv_module *

use std/assert
use std/testing *

# It's important to use random keys and to clean-up
# after since the user running these tests may have
# either an existing local stor or universal db.

@test
def simple-local-set [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)

    kv set $key 42
    let actual = (kv get $key)
    let expected = 42
    assert equal $actual $expected

    kv drop $key | ignore
}

@test
def local-pipeline_set_returns_value [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    let key = (random uuid)
    let actual = (42 | kv set $key)
    let expected = 42
    assert equal $actual $expected

    let actual = (kv get $key)
    let expected = 42
    assert equal $actual $expected

    kv drop $key | ignore
}

@test
def local-multiple_assignment [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    let key1 = (random uuid)
    let key2 = (random uuid)
    let key3 = (random uuid)

    "test value" | kv set $key1 | kv set $key2 | kv set $key3
    let expected = "test value"
    assert equal (kv get $key1) $expected
    assert equal (kv get $key2) $expected
    assert equal (kv get $key3) $expected
    assert equal (kv get $key3) (kv get $key1)

    kv drop $key1
    kv drop $key2
    kv drop $key3
}

@test
def local-transpose_to_record [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    let key1 = (random uuid)
    let key2 = (random uuid)
    let key3 = (random uuid)

    "test value" | kv set $key1 | kv set $key2 | kv set $key3

    let record = (kv list | transpose -dr)
    let actual = ($record | select $key1)
    let expected = { $key1: "test value" }

    assert equal $actual $expected

    kv drop $key1
    kv drop $key2
    kv drop $key3
}

@test
def local-using_closure [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    let name_key = (random uuid)
    let size_key = (random uuid)

    ls
    | kv set $name_key { get name }
    | kv set $size_key { get size }

    let expected = "list<string>"
    let actual = (kv get $name_key | describe)
    assert equal $actual $expected

    let expected = "list<filesize>"
    let actual = (kv get $size_key | describe)
    assert equal $actual $expected

    kv drop $name_key
    kv drop $size_key
}

@test
def local-return-entire-list [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    let key1 = (random uuid)
    let key2 = (random uuid)

    let expected = 'value1'
    $expected | kv set $key1

    let actual = (
        'value2'
        | kv set --return all $key2   # Set $key2, but return the entire kv store
        | transpose -dr  # Convert to record for easier retrieval
        | get $key1      # Attempt to retrieve key1 (set previously)
    )

    assert equal $actual $expected
    kv drop $key1
    kv drop $key2
}

@test
def local-return_value_only [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    let key = (random uuid)

    let expected = 'VALUE'
    let actual = ('value' | kv set -r v $key {str upcase})

    assert equal $actual $expected

    kv drop $key

}

@test
def universal-simple_set [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    $env.NU_UNIVERSAL_KV_PATH = (mktemp -t --suffix .sqlite3)

    let key = (random uuid)

    kv set -u $key 42
    let actual = (kv get -u $key)
    let expected = 42
    assert equal $actual $expected

    kv drop -u $key | ignore
    rm $env.NU_UNIVERSAL_KV_PATH
}

@test
def universal-pipeline_set_returns_value [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    $env.NU_UNIVERSAL_KV_PATH = (mktemp -t --suffix .sqlite3)

    let key = (random uuid)
    let actual = (42 | kv set -u $key)
    let expected = 42
    assert equal $actual $expected

    let actual = (kv get -u $key)
    let expected = 42
    assert equal $actual $expected

    kv drop -u $key | ignore
    rm $env.NU_UNIVERSAL_KV_PATH
}

@test
def universal-multiple_assignment [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    $env.NU_UNIVERSAL_KV_PATH = (mktemp -t --suffix .sqlite3)

    let key1 = (random uuid)
    let key2 = (random uuid)
    let key3 = (random uuid)

    "test value" | kv set -u $key1 | kv set -u $key2 | kv set -u $key3
    let expected = "test value"
    assert equal (kv get -u $key1) $expected
    assert equal (kv get -u $key2) $expected
    assert equal (kv get -u $key3) $expected
    assert equal (kv get $key3) (kv get $key1)

    kv drop -u $key1
    kv drop -u $key2
    kv drop -u $key3
    rm $env.NU_UNIVERSAL_KV_PATH
}

@test
def universal-transpose_to_record [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    $env.NU_UNIVERSAL_KV_PATH = (mktemp -t --suffix .sqlite3)

    let key1 = (random uuid)
    let key2 = (random uuid)
    let key3 = (random uuid)

    "test value" | kv set -u $key1 | kv set -u $key2 | kv set -u $key3

    let record = (kv list -u | transpose -dr)
    let actual = ($record | select $key1)
    let expected = { $key1: "test value" }

    assert equal $actual $expected

    kv drop -u $key1
    kv drop -u $key2
    kv drop -u $key3
    rm $env.NU_UNIVERSAL_KV_PATH
}

@test
def universal-using_closure [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    $env.NU_UNIVERSAL_KV_PATH = (mktemp -t --suffix .sqlite3)

    let name_key = (random uuid)
    let size_key = (random uuid)

    ls
    | kv set -u $name_key { get name }
    | kv set -u $size_key { get size }

    let expected = "list<string>"
    let actual = (kv get -u $name_key | describe)
    assert equal $actual $expected

    let expected = "list<filesize>"
    let actual = (kv get -u $size_key | describe)
    assert equal $actual $expected

    kv drop -u $name_key
    kv drop -u $size_key
    rm $env.NU_UNIVERSAL_KV_PATH
}

@test
def universal-return-entire-list [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    $env.NU_UNIVERSAL_KV_PATH = (mktemp -t --suffix .sqlite3)

    let key1 = (random uuid)
    let key2 = (random uuid)

    let expected = 'value1'
    $expected | kv set -u $key1

    let actual = (
        'value2'
        | kv set -u --return all $key2   # Set $key2, but return the entire kv store
        | transpose -dr  # Convert to record for easier retrieval
        | get $key1      # Attempt to retrieve key1 (set previously)
    )

    assert equal $actual $expected
    kv drop --universal $key1
    kv drop --universal $key2
    rm $env.NU_UNIVERSAL_KV_PATH
}

@test
def universal-return_value_only [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)
    $env.NU_UNIVERSAL_KV_PATH = (mktemp -t --suffix .sqlite3)

    let key = (random uuid)

    let expected = 'VALUE'
    let actual = ('value' | kv set --universal -r v $key {str upcase})

    assert equal $actual $expected

    kv drop --universal $key
    rm $env.NU_UNIVERSAL_KV_PATH
}

@test
def simple-local-set-table [] {
    if ('sqlite' not-in (version).features) { return }

    let key = (random uuid)

    kv set -t foo $key 42
    let actual = (kv get -t foo $key)
    let expected = 42
    assert equal $actual $expected

    kv drop -t foo $key | ignore
}