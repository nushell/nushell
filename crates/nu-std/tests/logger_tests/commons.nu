export def now [] {
    date now | format date "%Y-%m-%dT%H:%M:%S%.3f"
}

export def format-message [
    message: string,
    format: string
    prefix: string,
    ansi
    --context: record
] {

    let context = $context
        | default {}
        | transpose k v
        | each {|e| $'($e.k)="($e.v)"'}
        | str join ' '

    [
        ["%MSG%" $message]
        ["%DATE%" (now)]
        ["%LEVEL%" $prefix]
        ["%CONTEXT%" $context]
        ["%ANSI_START%" $ansi]
        ["%ANSI_STOP%" (ansi reset)]
    ] | reduce --fold $format {
        |it, acc| $acc | str replace --all $it.0 $it.1
    }
}
