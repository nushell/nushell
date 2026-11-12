$env.Path = if ($env | columns | "VENV_OLD_PATH" in $in) {
    $env.VENV_OLD_PATH
} else {
    $env.PATH
}
