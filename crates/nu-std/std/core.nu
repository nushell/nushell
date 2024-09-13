use dt.nu [datetime-diff, pretty-print-duration]

# Print a banner for nushell with information about the project
export def banner [] {
let dt = (datetime-diff (date now) 2019-05-10T09:59:12-07:00)
$"(ansi green)     __  ,(ansi reset)
(ansi green) .--\(\)°'.' (ansi reset)Welcome to (ansi green)Nushell(ansi reset),
(ansi green)'|, . ,'   (ansi reset)based on the (ansi green)nu(ansi reset) language,
(ansi green) !_-\(_\\    (ansi reset)where all data is structured!

Please join our (ansi purple)Discord(ansi reset) community at (ansi purple)https://discord.gg/NtAbbGn(ansi reset)
Our (ansi green_bold)GitHub(ansi reset) repository is at (ansi green_bold)https://github.com/nushell/nushell(ansi reset)
Our (ansi green)Documentation(ansi reset) is located at (ansi green)https://nushell.sh(ansi reset)
(ansi cyan)Tweet(ansi reset) us at (ansi cyan_bold)@nu_shell(ansi reset)
Learn how to remove this at: (ansi green)https://nushell.sh/book/configuration.html#remove-welcome-message(ansi reset)

It's been this long since (ansi green)Nushell(ansi reset)'s first commit:
(pretty-print-duration $dt)

Startup Time: ($nu.startup-time)
"
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
