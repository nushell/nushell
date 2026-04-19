# Wrapper over the terminal OSC 9;4 for progress bars. Your terminal application may not support any/some of the commands.

# Set progress bar with a percentage
export def set [
    percentage: int # percentage to set, must be between 0 and 100
] {
    print -n $"(ansi osc)9;4;1;($percentage)(ansi st)"
}

# Set progress bar with an index and a maximum value
export def set-idx [
    index: int # the current index
    total: int # the total value
] {
    print -n $"(ansi osc)9;4;1;($index / $total * 100 | into int)(ansi st)"
}

# Set progress bar to indeterminate
export def indeterminate [
] {
    print -n $"(ansi osc)9;4;3(ansi st)"
}

# Clear progress bar
export def clear [
] {
    print -n $"(ansi osc)9;4;0(ansi st)"
}

# Pause progress bar
export def pause [
] {
    print -n $"(ansi osc)9;4;4(ansi st)"
}

# Error progress bar
export def error [
    error_code: int = 0 # Error code to set in the progress bar
] {
    print -n $"(ansi osc)9;4;2;($error_code)(ansi st)"
}
