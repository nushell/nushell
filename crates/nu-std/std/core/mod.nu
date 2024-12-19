use std/dt [datetime-diff, pretty-print-duration]

# Print a banner for nushell with information about the project
export def banner [
    --short    # Only show startup time
] {
let dt = (datetime-diff (date now) 2019-05-10T09:59:12-07:00)
let ver = (version)

let startup_time = $"('Startup Time: ' | ansi gradient --fgstart '0x20a000' --fgend '0x20ff00')($nu.startup-time)"

let banner_msg = match $short {
    true => $"($startup_time)(char newline)"

    false => $"(ansi green)     __  ,(ansi reset)
(ansi green) .--\(\)°'.' (ansi reset)Welcome to (ansi green)Nushell(ansi reset),
(ansi green)'|, . ,'   (ansi reset)based on the (ansi green)nu(ansi reset) language,
(ansi green) !_-\(_\\    (ansi reset)where all data is structured!

Version: (ansi green)($ver.version) \(($ver.build_os)\)
Please join our (ansi purple)Discord(ansi reset) community at (ansi purple)https://discord.gg/NtAbbGn(ansi reset)
Our (ansi green_bold)GitHub(ansi reset) repository is at (ansi green_bold)https://github.com/nushell/nushell(ansi reset)
Our (ansi green)Documentation(ansi reset) is located at (ansi green)https://nushell.sh(ansi reset)
(ansi cyan)Tweet(ansi reset) us at (ansi cyan_bold)@nu_shell(ansi reset)
Learn how to remove this at: (ansi green)https://nushell.sh/book/configuration.html#remove-welcome-message(ansi reset)

It's been this long since (ansi green)Nushell(ansi reset)'s first commit:
(pretty-print-duration $dt)

($startup_time)
"
}

match $env.config?.use_ansi_coloring? {
    false => { $banner_msg | ansi strip }
    _ => $banner_msg
}
}

# Return the current working directory
export def pwd [
    --physical (-P) # resolve symbolic links
] {
    if $physical {
        $env.PWD | path expand
    } else {
        $env.PWD
    }
}
