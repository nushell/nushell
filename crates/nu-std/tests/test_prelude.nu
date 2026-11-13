use std/testing *
use std/assert

@test
def banner [] {
    use std/prelude
    assert ((prelude banner | lines | length) == 16)
}

@test
def banner_short_is_only_the_startup_time [] {
    use std/prelude
    let short = prelude banner --short | ansi strip | lines
    assert equal ($short | length) 1
    assert str contains $short.0 "Startup Time:"
}

@test
def banner_short_skips_the_welcome_message [] {
    use std/prelude
    # `debug profile` lists the calls made. The full banner checks that it reports these two.
    let full = debug profile { prelude banner } --max-depth 10 | get source
    assert ("version" in $full)
    assert ("datetime-diff" in $full)
    let short = debug profile { prelude banner --short } --max-depth 10 | get source
    assert ("version" not-in $short)
    assert ("datetime-diff" not-in $short)
}
