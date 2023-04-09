# Contributing

Welcome to the Nushell standard library and thank you for considering contributing!

## Ideas for the standard library

If you've got a great idea, or just want to contribute to open source
by working on the Nushell standard library, 
we invite you to talk to the team before you start coding. 
You'll find we're friendly, passionate about Nushell and always open to new ideas!  

You'll generally find the team members on
[Discord standard-library channel](https://discord.com/channels/601130461678272522/1075541668922658868), 
and can have preliminary discussions there to clarify the issues involved.

You can open a [Github issue](https://github.com/nushell/nushell/issues/new/choose) 
to have a more focused discussion of your idea.  

Generally, we think the standard library should contain items that are 
relevant to most/all Nushell users regardless of the application space they're working in. 
If your idea isn't quite so broadly applicable, 
consider publishing it in [nu_scripts](https://github.com/nushell/nu_scripts).

Preliminary discussions should focus on the *user benefit* your idea would provide.  
How many users will be affected by your idea, how would (a good implementation of) it 
improve their Nushell experience?
Given consensus on the user benefit, the team will be motivated to 
help you create, deploy and maintain a solution long term.

## Lifecycle of a change

1. Verify the team thinks your idea is potentially relevant and useful, as above.
1. If it's more than a simple bug fix, open a placeholder PR 
as soon as you get started and mark it as work in progress (WIP).
This will alert other contributors that you're working in this area and 
advertise roughly what scope of changes you're thinking of.
See [below](the_pr) for details.
1. Get things working in your local development environment.  
If you have questions along the way, you can post a question in your PR 
or have a more casual discussion with Nushell fans on 
[Discord implementation-chat channel](https://discord.com/channels/601130461678272522/615962413203718156)
1. When you get to an appropriate state of doneness, push your changes to the PR and remove the WIP flag.
2. Team members and other contributors will then review your PR.  
Respond to any review comments they raise and resolve them one way or another.
(Not all comments demand you make a change!)
1. When you and the team are comfortable with the PR, 
a team member will merge it into the repo and you can delete your working branch.
2. If you've added a whole new command or made a breaking change, 
(strongly) consider writing up a section for the release notes.  
Currently, release notes are maintained in a different repo, [nushell.github.io](https://github.com/nushell/nushell.github.io). 
Make your change in a local clone of that repo and submit a PR to the release notes repo to get it integrated.

## Developing

### Setup
To work on the standard library, you won't necessarily need to compile or debug the interpreter itself, 
so you might not need to install the Rust toolchain. 
But if you want to do that, see [nushell CONTRIBUTING.md](https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md).

1. Clone the Nushell repo containing the standard library.  
Currently, that's the [Nushell interpreter source repo](https://github.com/nushell/nushell).
    ```
    git clone https://github.com/nushell/nushell
    ```

2. Arrange a runnable Nushell interpreter  
  Either install a [pre-built distribution](https://github.com/nushell/nushell/releases), or build from source:
    ```
    git clone https://github.com/nushell/nushell
    cd nushell
    cargo run
    ```
1. Focus your IDE on the standard library folder  
(Currently, it is a Rust crate within the Nushell repo)

    ```
    cd /path/to/nushell/repo
    code .  # opinionated: we love vscode!
    (open folder crates/nu-std in IDE)
   ```
### The PR
If your idea is mostly a bug fix, you might not open a PR till you've got a fix (nearly)ready to go.  

But if you're considering new commands or substantial or (even simple) breaking changes, 
it's good to advertise your proposed work to other collaborators for their awareness 
and to solicit their early feedback. 
Often, this feedback will result in a better and more maintainable solution long term, 
so give your fellow collaborators the opportunity.  Someday, they'll return the favor...

To open a PR before you've got code ready to go:
  1. create a feature branch in your fork of the repo
  2. make a trivial code change and push it to Github
  4. open a PR on the standard library repo
  5. **mark it as a work in progress (WIP)**  
This signals that you are not yet asking to have the PR formally reviewed and merged.
  6. provide a preliminary description of your proposed change.     
Your description should include the external interface for your feature, 
a draft of the command arguments and signature, 
new/changed environment variables and any other user-visible changes.

### Design considerations
The standard library consists mostly of Nushell custom commands and their associated environment variables.  
For background on these Nushell features, see 
[Modules chapter of the Nushell book](https://www.nushell.sh/book/modules.html). 
Existing code in the standard library provides pretty good examples to work from.  

1. Create / update the source and test files for a custom command `foo` as follows:
   ```shell
   # source file 
   /path/to/nushell/crates/nu-std/lib/foo.nu  
   # corresponding unit test file
   /path/to/nushell/crates/nu-std/tests/test_foo.nu
   ```
   Thou shalt provide unit tests to cover your changes.   
2. Typically, the source file will implement multiple subcommands and possibly a main command as well.  
   For an example of a custom command which does not modify environment variables (but may reference them), see:
   ```shell
   /path/to/nushell/crates/nu-std/lib/assert.nu
   ```
   Note the use of `export def` to define the subcommand.  

   For an example of a custom command which *does* modify environment variables, see:
   ```shell
   /path/to/nushell/crates/nu-std/lib/dirs.nu
   ```
   Note the use of `export def-env` to define the subcommand, 
   the use of `export-env {}` to initialize the environment variable and  `let-env` to update it. 

1. A `foo` command will be exposed to the user as `std foo` (at a minimum).  
To enable this, update file `/path/to/nushell/crates/nu-std/lib/mod.rs` and add this code:
   ```
   export use crates/nu-std/lib/foo.nu *    # command doesn't update environment
   export-env {
          use crates/nu-std/lib/bar.nu *    # command *does* update environment
   }
   ```
   Note use of `use *` to hoist all subcommands into `mod.nu` and thus into the `std` namespace.

2. Some commands in standard library are also hoisted into the top level, non-prefixed namespace, 
so a `foo` command can be invoked without prefix as `foo`.  
To do this, modify `/path/to/nushell/crates/nu-std/src/lib.rs`, 
find the initialization of the "prelude" at line 70 or thereabouts and add `(foo, foo)`.  
(This code may be restructured soon: if you can't find it, check with the team on Discord.)  
Note that you will need to recompile the Nushell interpreter to test this change, 
see [Nushell Contributing#Setup](https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md#setup).

More general guidelines:

1. Ensure your custom command provides useful help.  
This is done with comments before the command and within the argument declaration.  
1. Use `error make` to report can't-proceed errors to user, not `log error`.  
2. Use `log info` to provide a verbose progress messages that the user can optionally enable for troubleshooting. 
e.g: 
    ```shell
    NU_LOG_LEVEL=INFO foo # verbose messages from command foo
    ```
3. Use `assert` in unit tests to check for and report failures.  

### Useful Commands

- Run standard library tests, logging only test failures (meaning that no output is the desired outcome.):  
  
  ```shell
  cd /path/to/nushell
  NU_LOG_LEVEL=ERROR nu "crates/nu-std/tests/run.nu"
  ```
  Change 'ERROR' to 'INFO' or 'DEBUG' for increasing verbousity.

- Run tests for a specific command, e.g, command `foo`

  ```shell
  cd /path/to/nushell
  NU_LOG_LEVEL=INFO nu "crates/nu-std/tests/test_foo.nu"
  ```

- Run a custom command with additional logging (assuming you have instrumented
the command with `log <level>`, as we recommend.)

  ```shell
  NU_LOG_LEVEL=INFO std foo bar bas # verbose
  NU_LOG_LEVEL=DEBUG std foo bar bas # very verbose  
  ```
- Build and run Nushell (e.g, if you modify the prelude):

  ```shell
  cargo run
  ```
## Git commit and repo conventions
The standard library project uses the same protocols and conventions 
for squashing git commits and handling github PRs as the core Nushell project. 
Please see [nushell Contributing#git_etiquette](https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md#git-etiquette) for details.
