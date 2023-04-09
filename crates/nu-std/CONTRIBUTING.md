[[doc issues and concerns noted in double brackets like this.  These should be addressed before merging the PR]]
# Contributing

Welcome to the Nushell standard library and thank you for considering contributing!

## Ideas for the standard library

If you've got a great idea, or just want to contribute to open source by working on the Nushell standard library, we invite you to talk to the team before you start coding.  You'll find we're friendly, passionate about Nushell and always open to new ideas!  

You'll generally find the team members on [Discord standard-library channel](https://discord.com/channels/601130461678272522/1075541668922658868), and can have preliminary discussions there to clarify the issues involved.

You can open a [Github issue](https://github.com/nushell/nushell/issues/new/choose) to have a more focused discussion of your idea.  

Generally, we think the standard library should contain items that are 
relevant to most/all Nushell users regardless of the application space they're working in.  If your idea isn't quite so broadly applicable, consider publishing it in [nu_scripts](https://github.com/nushell/nu_scripts).

Preliminary discussions should focus on the *user benefit* your idea would provide.  How many users will be affected by your idea, how will (a good implementation of) it improve their Nushell experience and how big a deal with they think it is? Given consensus on the user benefit, the team will be motivated to help you create, deploy and maintain a solution long term.

## Lifecycle of a change

1. Verify the team thinks your idea is potentially relevant and useful, as above.
2. [[We don't often do things this way at the moment, but it's a good way to manage the collaboration, so recommended here]]  
If it's more than a simple bug fix, open a placeholder PR as soon as you get started and mark it as work in progress (WIP).  This will alert other contributors that that you're working in this area and advertise roughly what scope of changes you're thinking of.  See [below](the_pr) for details.
1. Get things working in your local development environment.  
If you have questions along the way, you can post a question in your PR or have a more casual discussion with Nushell fans on [Discord implementation-chat channel](https://discord.com/channels/601130461678272522/615962413203718156)
2. When you get to an appropriate state of doneness, push your changes to the PR and remove the WIP flag.
3. Team members and other contributors will then review your PR (maybe even before you remove the WIP flag!)  Respond to any review comments they raise and resolve them one way or another. (Not all comments demand you make a change!)
4. When you and the team are comfortable with the PR, a team member will merge it into the repo and you can delete your working branch.
5. If you've added a whole new command or made a breaking change, (strongly) consider writing up a section for the release notes.  Currently, release notes are maintained in [a different repo, nushell.github.io](https://github.com/nushell/nushell.github.io).  Make your change in your clone of that repo and submit a PR to the release notes repo to get it integrated.

## Developing
To work on the standard library, you don't necessarily need to compile or debug the interpreter itself, so you might not need to install the Rust toolchain. But if you want to do that, see [nushell CONTRIBUTING.md](https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md).

### Setup

1. Clone the Nushell repo containing the standard library.  Currently, that's the [Nushell interpreter source repo](https://github.com/nushell/nushell).

    ```shell
    git clone https://github.com/nushell/nushell
    ```
2. Arrange a runnable Nushell interpreter  
  Either install a [pre-built distribution](https://github.com/nushell/nushell/releases), or build from source:
    ```shell
    git clone https://github.com/nushell/nushell
    cd nushell
    cargo run
    ```
1. Focus your IDE on the standard library folder  
(Currently, it is a Rust crate within the Nushell repo)

    ```shell
    cd /path/to/nushell/repo
    cd crates/nu-std
    code .  # opinionated: we love vscode!
    ```

### The PR
If your idea is mostly a bug fix, you might not open a PR till you've got a fix (nearly)ready to go.  

But if you're considering new commands or substantial or (even simple) breaking changes, it's good to advertise your proposed work to other collaborators for their awareness and to solicit their early feedback.  Often, this feedback will result in a better and more maintainable solution long term, so give your fellow collaborators the opportunity.  Someday, they'll return the favor...

To open a PR before you've got code ready to go:
  1. create a feature branch in your fork of the repo
  2. make a trivial code change and push it to Github
  4. open a PR on the standard library repo
  5. **mark it as a work in progress (WIP)**  
This avoids running CI in the nushell repo (until you remove the flag)
  6. provide a preliminary description of your proposed change.     
Your description should include the external interface for your feature, a draft of the command arguments and signature, new/changed environment variables and any other user-visible changes.

### Design considerations

The existing code in the standard library is a pretty good sample to start from.  
[[idea: consider creating a `skeleton` custom command to act as a template to be cloned into a new command, or documenting one of the existing ones as the golden master.]]

1. Standard library consists of custom commands and environment variables packaged in modules and exported into the user's environment.  For background on these Nushell features, see [Modules chapter of the Nushell book](https://www.nushell.sh/book/modules.html).
2. Ensure your custom command provides useful help.  This is done with comments before the command and within the argument declaration.  
[[I want to refer user to documentation for this, but can't find any.  I'll write some if you'll point me at sources to study.]]
3. Use `error make` to report can't-proceed errors to user, not `log error`.  Use `log info` to provide a capability for user to enable "verbose" progress messages.  Use `assert` to report failures in unit tests.  
4. All objects in standard library are exposed in `std` namespace, so if the command is `foo`, it is referenced as `std foo ...`  This requires a `use` statement in `std.nu` which references your (new) module. See [[tbd, waiting #8815 to land]] for an example
5. Some commands in standard library are also hoisted into the top level namespace, so if the command is again `foo`, it is invoked as `foo` without prefix.  You can do this for your new command by adding it to the "prelude" at `/path/to/nushell/crates/nu-std/src/lib.rs` line 70, or thereabouts.  
[[at the least, refactor this code and add comments to guide dev]].  
Note that you will need to recompile the Nushell interpreter to test this change, see [Nushell Contributing#Setup](https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md#setup).
1. Ensure your additions or changes are covered by standard library unit tests.  Tests for a `foo` command would be found in `test_foo.nu` and run as described below.

### Useful Commands


- Run a custom command with additional logging (assuming you have instrumented the command with `log <level>`, as we recommend.)

  ```shell
  NU_LOG_LEVEL=INFO std foo bar bas # verbose
  NU_LOG_LEVEL=DEBUG std foo bar bas # very verbose  
  ```

- Run standard library tests, logging only test failures (meaning that no output is the desired outcome.):  
[[fix paths]]

  
  ```shell
  cd /path/to/nushell
  NU_LOG_LEVEL=ERROR nu "crates/nu-std/tests.nu"
  ```

  Change 'ERROR' to 'INFO' or 'DEBUG' for increasing verbousity.

- Run tests for a specific command, e.g, command `foo`

  ```shell
  cd /path/to/nushell
  NU_LOG_LEVEL=INFO nu "crates/nu-std/tests_foo.nu"
  ```

- Build and run Nushell (e.g, if you modify the prelude):

  ```shell
  cargo run
  ```


### Debugging Tips

[[Nushell needs better script debugging primitives before there's much to say here.  e.g a `debug --pause` that prompts for debug commands which can examine/modify variables in scope, show stack?]]

## Project and repo conventions
The standard library project uses the same protocols and conventions for structuring the git commits and github PRs as the core Nushell project.  Please see [nushell Contributing#git_etiquette](https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md#git-etiquette) for details.
