# Frequently asked question for developers

Let's collect some questions a lot of Nushell contributors have.
- How do I do....?
- Why do I need to do certain things a certain way?

Let's keep the answers concise and up to date (or general enough) to remain relevant

## How do I properly test my feature or bugfix?
TODO (Probably fork out into its own file)

## I want to report an error to the user

Approximate flow:

1. Are you reporting the error in the parser/static checking phase?
    - Use `nu_protocol::ParseError` variants
    - Follow the logic used in the context as we need to collect multiple errors for a good IDE experience
2. Pick the right `nu_protocol::ShellError` variant
    - Does a matching existing variant fit your need? (go to references of the `ShellError` variant for inspiration)
    - Check what context the [`miette`](https://docs.rs/miette) macros add during formatting! (go to definition of `ShellError`)
    - If it is a one-of specific error, consider using a generic variant
    - Else add a new class of errors
        - add the necessary `Span` information
        - general shared error text, to inform and point to a resolution
        - dynamic information gathered from the error site
        - Don't use a tuple enum variant, named structs going forward only!
3. Are you in a command?
    - `return Err(ShellError::...)` and you're done in a `Command::run`
4. Do you want to report a warning but not stop execution?
    - **NEVER** `println!`, we can write to stderr if necessary but...
    - good practice: `nu_protocol::report_error::report_error` or `report_error_new`
        - depending on whether you have access to a `StateWorkingSet`
    - if only relevant to in the field debugging: `log`-crate macros.

## How do I check an environment variable?
TODO

## WTF is `PipelineMetadata`?
TODO
