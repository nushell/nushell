use std/dt [datetime-diff, pretty-print-duration]

# A value that may be absent — `Option.some <value>` or `Option.none`.
#
# `Option<T>` is a generic enum type with two variants: `some` carries
# a payload of type `T`; `none` carries nothing. It replaces `null` in
# positions where absence is an expected outcome, so that handling it
# is visible in signatures and checked by `match`.
#
# Constructors may be written bare or with explicit type arguments:
# `Option.some 5` infers `Option<int>` from its payload, while
# `Option<int>.some 3` and `Option<string>.none` pin `T` directly.
# `Option.none` (and any inferred `Option<any>`) is accepted wherever
# an `Option` is expected, regardless of `T`; a concrete instantiation
# accepts only its own payloads, so `Option<int>.some "x"` is a parse
# error.
#
# `match` on an `Option` is checked for exhaustiveness:
#
# ```nu
# match $x {
#     Option.some $v => $"got ($v)"
#     Option.none => "nothing here"
# }
# ```
#
# Like all enum values, an `Option` lowers to a base record —
# `{kind: "some", payload: <value>}` for `some` and `{kind: "none"}`
# for `none` — for display, serialization, and cell-path access.
# `Option.from-record` rebuilds an `Option` from its base record.
#
# Helpers live in `std/option` (`option map`, `option unwrap-or`, ...).
export enum Option<T> { some: T, none }

# A computation that may fail — `Result.ok <value>` or `Result.err <error>`.
#
# `Result<T, E>` is a generic enum type for recoverable errors: `ok`
# carries a success payload of type `T`; `err` carries an error payload
# of type `E`. A signature like `def parse []: nothing -> Result<record,
# string>` declares failure as an ordinary typed outcome that callers
# handle through `match`, rather than as an exception.
#
# `result try { }` converts `try`/`catch` into a `Result`: the block's
# value becomes `ok`, and a raised error becomes `err` carrying the
# catch-style error record (`msg`, `rendered`, `details`, `raw`).
# The `raw` field holds the live error value — using it in a pipeline
# rethrows the original error, so `reject raw` before serializing an
# `err` produced this way.
#
# Constructors may be written bare or with explicit type arguments:
# `Result.err "x"` infers `Result<any, string>` from its payload, while
# `Result<int, string>.ok 3` pins both. An `any` argument matches any
# instantiation in that position; a concrete argument accepts only its
# own payload. `match` on a `Result` must cover both `ok` and `err`.
# Values lower to `{kind: "ok"|"err", payload: <value>}` base records
# (see `Option` above).
#
# Helpers live in `std/result` (`result try`, `result map-err`,
# `result unwrap-or`, ...).
export enum Result<T, E> { ok: T, err: E }

# The `Option`/`Result` helper modules — `option map`,
# `option unwrap-or`, `result try`, `result map-err`, ... —
# so `use std/prelude` brings the whole toolkit.
export use std/option
export use std/result

# Print a banner for Nushell with information about the project
@category "default"
@search-terms "welcome" "startup"
export def banner [
    --short            # Only show the startup time
    --no-startup-time  # Show the welcome message without the startup time
] {
let foreground = $env.config.color_config?.banner_foreground? | default "attr_normal"
let highlight1 = $env.config.color_config?.banner_highlight1? | default "green"
let highlight2 = $env.config.color_config?.banner_highlight2? | default "purple"
let dt = (datetime-diff (date now) 2019-05-10T09:59:12-07:00)
let ver = (version)
let startup_time = $"(ansi $highlight1)(ansi attr_bold)Startup Time: (ansi reset)(ansi $foreground)($nu.startup-time)(ansi reset)"

let welcome = $"(ansi $highlight1)     __  ,(ansi reset)
(ansi $highlight1) .--\(\)°'.' (ansi reset)(ansi $foreground)Welcome to (ansi $highlight1)Nushell(ansi reset)(ansi $foreground),(ansi reset)
(ansi $highlight1)'|, . ,'   (ansi reset)(ansi $foreground)based on the (ansi $highlight1)nu(ansi reset)(ansi $foreground) language,(ansi reset)
(ansi $highlight1) !_-\(_\\    (ansi reset)(ansi $foreground)where all data is structured!

(ansi $foreground)Version: (ansi $highlight1)($ver.version) \(($ver.build_target)\)(ansi reset)
(ansi $foreground)Please join our (ansi $highlight2)Discord(ansi reset)(ansi $foreground) community at (ansi $highlight2)https://discord.gg/NtAbbGn(ansi reset)
(ansi $foreground)Our (ansi $highlight1)(ansi attr_bold)GitHub(ansi reset)(ansi $foreground) repository is at (ansi $highlight1)(ansi attr_bold)https://github.com/nushell/nushell(ansi reset)
(ansi $foreground)Our (ansi $highlight2)Documentation(ansi reset)(ansi $foreground) is located at (ansi $highlight2)https://nushell.sh(ansi reset)
(ansi $foreground)And the (ansi $highlight1)Latest Nushell News(ansi reset)(ansi $foreground) at (ansi $highlight1)https://nushell.sh/blog/(ansi reset)
(ansi $foreground)Learn how to remove this at: (ansi $highlight2)https://nushell.sh/book/configuration.html#remove-welcome-message(ansi reset)

(ansi $foreground)It's been this long since (ansi $highlight1)Nushell(ansi reset)(ansi $foreground)'s first commit:(ansi reset)
(ansi $foreground)(pretty-print-duration $dt)
"

# The REPL prints the welcome message before the startup hooks run and the startup time right
# before the first prompt, once it is final.
let banner_msg = if $short {
    $"($startup_time)(char eol)"
} else if $no_startup_time {
    $welcome
} else {
    $"($welcome)(char eol)($startup_time)(ansi reset)(char eol)"
}

match (config use-colors) {
    false => { $banner_msg | ansi strip }
    _ => $banner_msg
}
}

# Return the current working directory
@category "default"
export def pwd [
    --physical (-P) # Resolve symbolic links
] {
    if $physical {
        $env.PWD | path expand
    } else {
        $env.PWD
    }
}
