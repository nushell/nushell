# Contributing

Welcome to the Nushell standard library and thank you for considering
contributing!

## Ideas for the standard library
If you've got a great idea, or just want to contribute to open source by
working on the Nushell standard library, we invite you to talk to the team
before you start coding. You'll find we're friendly, passionate about Nushell
and always open to new ideas!

You'll generally find the team members on
[Discord `#standard-library` channel][discord#standard-library] and can have
preliminary discussions there to clarify the issues involved.

You can open a [GitHub issue][new-issue] to have a more focused discussion of
your idea.

Generally, we think the standard library should contain items that are relevant
to most/all Nushell users regardless of the application space they're working
in. If your idea isn't quite so broadly applicable, consider publishing it in
[`nu_scripts`].

Preliminary discussions should focus on the *user benefit* your idea would
provide.

How many users will be affected by your idea, how much would it help them solve
a problem or work more productively? Given consensus on the user benefit, the
team will be motivated to help you create, deploy and maintain a solution long
term.

## Lifecycle of a change
1. Verify the team thinks your idea is potentially relevant and useful, as
  above.
1. If it's more than a simple bug fix, open a placeholder PR as soon as you get
  started and [set it to draft status][github_draft_pr].  
  This will alert other contributors that you're working in this area and let
  you advertise roughly what scope of changes you're thinking of. See
  [below](#the-pr) for details.
1. Get things working in your local development environment.  
  If you have questions along the way, you can post a question in your PR or
  have a more casual discussion with Nushell fans on
  [Discord `#implementation-chat` channel][discord#implementation-chat].
1. When you get to an appropriate state of doneness, push your changes to the
  PR and remove the draft status.
1. Team members and other contributors will then review your PR.  
  Respond to any review comments they raise and address them one way or
  another. (Not all comments demand you make a change!)
1. When you and the team are comfortable with the PR, a team member will merge
  it into the repo and you can delete your working branch.
1. If you've added a whole new command or made a breaking change,
  (strongly) consider writing it up for the release notes.  
  Currently, release notes are maintained in a different repo,
  [`nushell.github.io`]. Make your change in a local clone of that repo and
  submit a PR to the release notes repo to get it integrated.

## Developing
(All paths below shown relative to the root folder of the git repository
containing the standard library.)

### Setup
0. Install the Rust toolchain and Nushell build tools.
  See [`nushell`'s `CONTRIBUTING.md`][`CONTRIBUTING.md`] for details. The
  standard library is tightly coupled to a particular version of Nushell
  interpreter, you need to be running that version to test your changes (unlike
  a "normal" script module library).
1. Clone the Nushell repo containing the standard library and create a feature
  branch for your development work.  
  Currently, that's the [Nushell interpreter source repo][`nushell`].  
  Once you set your working directory to the root of this repository, you'll
  generally leave it there throughout the session.
    ```shell
    git clone https://github.com/nushell/nushell
    cd nushell
    git checkout -b <featureBranch>
    ```
1. In your IDE, open the folder within the repository containing the standard
  library. The folder is currently `./crates/nu-std`, and it is a Rust crate,
  containing a `Cargo.toml` and subfolders:
    * `src/` (which contains the Rust code to load the standard library modules
      into memory for efficiency),
    * `lib` (which contains all the script module sources for the standard
      library),
    * `tests/` (unit tests for lib).

### The PR
Assuming you've already validated the need with other Nushell contributors,
you're focusing on design and implementation at this point. Share your thinking
all along the way!

You can open a [draft][github_draft_pr] pull request based on a small,
placeholder code change and use the PR comments to outline your design and user
interface. You'll get feedback from other contributors that may lead to a more
robust and perhaps more idomatic solution. The threads in the PR can be a
convenient reference for you when writing release notes and for others on the
team when researching issues.

> **Note**  
> the PR will not get final code review or be merged until you remove the draft
> status.

### Design considerations
The standard library consists of Nushell custom commands and their associated
environment variables, packaged in script modules underneath module `std`. For
background on scripts, custom commands and modules, see the
[Modules chapter of the Nushell book][book@modules].

To add a completely new module, for example, a `foo` command and some
`foo subcommand`s, you will be dealing with 2 new source files: the module
source itself (`./crates/nu-std/lib/foo.nu`) and a unit tests file
(`./crates/nu-std/tests/test_foo`); and will be modifying 1 or 2 existing files
(`./crates/nu-std/lib/mod.nu` and possibly `./crates/nu-std/src/lib.rs`). This
is described below:

1. Source for a custom command `foo` should go in `./crates/nu-std/lib/foo.nu`.
    * A source file will typically implement multiple subcommands and possibly
      a main command as well.  
      Use `export def` to make these names public to your users.
    * If your command is updating environment variables, you must use
      `export def --env` (instead of `export def`) to define the subcommand,
      `export-env {}` to initialize the environment variables and `$env.VAR = val` to
      update them. For an example of a custom command which modifies
      environment variables, see: `./crates/nu-std/lib/dirs.nu`.  
      For an example of a custom command which does *not* modify environment
      variables, see: `./crates/nu-std/lib/assert.nu`.
    * If your standard library module wishes to use a utility from another
      module of the standard library, for example `log info`, you need to
      import it directly from its module in the `use` statement.
      ```nushell
      ... your foo.nu ...
      export def mycommand [] {
        use log "log info"
        . . .
        log info "info level log message"
        . . .
      }
      ```
      This is `use log "log info"` rather than `use std "log info"` (which is
      the usual way commands are imported from the standard library) because
      your `foo` module is also a child module under `std`.
1. Unit tests for `foo` should go in `./crates/nu-std/tests/test_foo.nu`. Thou
  shalt provide unit tests to cover your changes.
    * Unit tests should use one of the `assert` commands to check a condition
      and report the failure in a standard format.
    * To import `assert` commands for use in your test, import them via
      `use std` (unlike the `use log` for your source code; the tests are not
      modules under `std`).  For example:
      ```nushell
      ... your test_foo.nu ...
      def test1 [] {
        use std
        . . .
        std assert greater $l $r
        . . .
        std assert $predicate
      }

      def test2 [] {
        use std ['assert greater' assert]
        . . .
        assert greater $l $r
        . . .
        assert $predicate
      }
      ```
      The choice of import style is up to you.
1. A `foo` command will be exposed to the user as `std foo` (at a minimum).  
  To enable this, update file `./crates/nu-std/lib/mod.nu` and add this code:
    ```nushell
    export use foo *    # command doesn't update environment
    export-env {
           use bar *    # command *does* update environment
    }
    ```
    The `use *` hoists the public definitions in `foo.nu` into `mod.nu` and
    thus into the `std` namespace.
1. Some commands from the standard library are also preloaded, so user can
  invoke them without explicit import via `use std ...`.  
  A command implemented as `std foo`, can be preloaded as a bare `foo`:
    * modify `./crates/nu-std/src/lib.rs`,
    * find the initialization of the "prelude" at line 90 or thereabouts
    * add `("foo", "foo")`  
    * or, to be preloaded as `std foo`, add `("std foo", "foo")`.

    (This code may be restructured soon: if you can't find it, check with the
    team on Discord.)  
    > **Note**  
    > that you will need to recompile the Nushell interpreter to test this
    > change, see the ["setup" section][`CONTRIBUTING.md`#setup] of Nushell's
    > `CONTRIBUTING.md`.

More design guidelines:

1. Ensure your custom command provides useful help.  
  This is done with comments before the `def` for the custom command.
1. Use `error make` to report can't-proceed errors to user, not `log error`.
1. Use `log info` to provide verbose progress messages that the user can
  optionally enable for troubleshooting. e.g:
    ```nushell
    NU_LOG_LEVEL=INFO foo # verbose messages from command foo
    ```
1. Use `assert` in unit tests to check for and report failures.

### Useful Commands
- Run all unit tests for the standard library:
  ```nushell
  cargo run -- -c 'use crates/nu-std/testing.nu; testing run-tests --path crates/nu-std'
  ```
  > **Note**  
  > this uses the debug version of NU interpreter from the same repo, which is
  > the usual development scenario.  
  > Log level 'ERROR' shows only failures (meaning no output is the desired
  > outcome).  
  > Log level 'INFO' shows progress by module and 'DEBUG' show each individual
  > test.
- Run all tests for a specific test module, e.g,
  `crates/nu-std/tests/test_foo.nu`
  ```nushell
  cargo run -- -c 'use crates/nu-std/testing.nu; testing run-tests --path crates/nu-std --module test_foo'
  ```
- Run a custom command with additional logging (assuming you have instrumented
  the command with `log <level>`, as we recommend.)
  ```nushell
  NU_LOG_LEVEL=INFO std foo bar bas # verbose
  NU_LOG_LEVEL=DEBUG std foo bar bas # very verbose
  ```
- Build and run Nushell (e.g, if you modify the prelude):
  ```nushell
  cargo run
  ```

## Git commit and repo conventions
The standard library project uses the same protocols and conventions
for squashing git commits and handling github PRs as the core Nushell project.
Please see the ["Git etiquette" section][`CONTRIBUTING.md`#git-etiquette] of
Nushell's `CONTRIBUTING.md` for details.

[github_draft_pr]: https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/changing-the-stage-of-a-pull-request
[discord#standard-library]: https://discord.com/channels/601130461678272522/1075541668922658868
[discord#implementation-chat]: https://discord.com/channels/601130461678272522/615962413203718156
[new-issue]: https://github.com/nushell/nushell/issues/new/choose
[`nushell`]: https://github.com/nushell/nushell
[`nu_scripts`]: https://github.com/nushell/nu_scripts
[`nushell.github.io`]: https://github.com/nushell/nushell.github.io
[`CONTRIBUTING.md`]: https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md
[`CONTRIBUTING.md`#setup]: https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md#setup
[`CONTRIBUTING.md`#git-etiquette]: https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md#git-etiquette
[book@modules]: https://www.nushell.sh/book/modules.html
