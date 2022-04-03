def "nu-complete test" [context: string, offset: int] {
    [ $"prev ($context)" "two", $"offset ($offset)" ]
}

export extern "kubectl something" [
    target?: string@"nu-complete test"
]