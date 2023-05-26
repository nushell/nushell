export def build-help-header [
    text: string
    --no-newline (-n): bool
] {
    let header = $"(ansi green)($text)(ansi reset):"

    if $no_newline {
        $header
    } else {
        $header ++ "\n"
    }
}
