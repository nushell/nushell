# Contributing

Welcome to Nushell and thank you for considering contributing!

## Table of contents
- [Proposing design changes](#proposing-design-changes)
- [Developing](#developing)
  - [Setup](#setup)
  - [Tests](#tests)
  - [Useful commands](#useful-commands)
  - [Debugging tips](#debugging-tips)
- [Git etiquette](#git-etiquette)
- [Our Rust style](#our-rust-style)
  - [Generally discouraged](#generally-discouraged)
  - [Things we want to get better at](#things-we-want-to-get-better-at)
- [License](#license)

## Proposing design changes

First of all, before diving into the code, if you want to create a new feature, change something significantly, and especially if the change is user-facing, it is a good practice to first get an approval from the core team before starting to work on it.
This saves both your and our time if we realize the change needs to go another direction before spending time on it.
So, please, reach out and tell us what you want to do.
This will significantly increase the chance of your PR being accepted.

The review process can be summarized as follows:
1. You want to make some change to Nushell that is more involved than simple bug-fixing.
2. Go to [Discord](https://discordapp.com/invite/NtAbbGn) or a [GitHub issue](https://github.com/nushell/nushell/issues/new/choose) and chat with some core team members and/or other contributors about it.
3. After getting a green light from the core team, implement the feature, open a pull request (PR) and write a concise but comprehensive description of the change.
4. If your PR includes any use-facing features (such as adding a flag to a command), clearly list them in the PR description.
5. Then, core team members and other regular contributors will review the PR and suggest changes.
6. When we all agree, the PR will be merged.
7. If your PR includes any user-facing features, make sure the changes are also reflected in [the documentation](https://github.com/nushell/nushell.github.io) after the PR is merged.
8. Congratulate yourself, you just improved Nushell! :-)

## Developing

### Setup

Nushell requires a recent Rust toolchain and some dependencies; [refer to the Nu Book for up-to-date requirements](https://www.nushell.sh/book/installation.html#build-from-source). After installing dependencies, you should be able to clone+build Nu like any other Rust project:

```bash
git clone https://github.com/nushell/nushell
cd nushell
cargo build
```

### Tests

It is a good practice to cover your changes with a test. Also, try to think about corner cases and various ways how your changes could break. Cover those in the tests as well.

Tests can be found in different places:
* `/tests`
* `src/tests`
* command examples
* crate-specific tests

Most of the tests are built upon the `nu-test-support` crate. For testing specific features, such as running Nushell in a REPL mode, we have so called "testbins". For simple tests, you can find `run_test()` and `fail_test()` functions.

### Useful Commands

As Nushell is build using a cargo workspace consisting of multiple crates keep in mind that you may need to pass additional flags compared to how you may be used to it from a single crate project.
Read cargo's documentation for more details: https://doc.rust-lang.org/cargo/reference/workspaces.html

- Build and run Nushell:

  ```shell
  cargo run
  ```

- Build and run with dataframe support.
  ```shell
  cargo run --features=dataframe
  ```

- Run Clippy on Nushell:

  ```shell
  cargo clippy --workspace -- -D warnings -D clippy::unwrap_used
  ```
  or via the `toolkit.nu` command:
  ```shell
  use toolkit.nu clippy
  clippy
  ```

- Run all tests:

  ```shell
  cargo test --workspace
  ```

  along with dataframe tests

  ```shell
  cargo test --workspace --features=dataframe
  ```
  or via the `toolkit.nu` command:
  ```shell
  use toolkit.nu test
  test
  ```

- Run all tests for a specific command

  ```shell
  cargo test --package nu-cli --test main -- commands::<command_name_here>
  ```

- Check to see if there are code formatting issues

  ```shell
  cargo fmt --all -- --check
  ```
  or via the `toolkit.nu` command:
  ```shell
  use toolkit.nu fmt
  fmt --check
  ```

- Format the code in the project

  ```shell
  cargo fmt --all
  ```
  or via the `toolkit.nu` command:
  ```shell
  use toolkit.nu fmt
  fmt
  ```

- Set up `git` hooks to check formatting and run `clippy` before committing and pushing:

  ```shell
  use toolkit.nu setup-git-hooks
  setup-git-hooks
  ```
  _Unfortunately, this hook isn't available on Windows._

### Debugging Tips

- To view verbose logs when developing, enable the `trace` log level.

  ```shell
  cargo run --release -- --log-level trace
  ```

- To redirect trace logs to a file, enable the `--log-target file` switch.
  ```shell
  cargo run --release -- --log-level trace --log-target file
  open $"($nu.temp-path)/nu-($nu.pid).log"
  ```

## Git etiquette

As nushell thrives on its broad base of volunteer contributors and maintainers with different backgrounds we have a few guidelines for how we best utilize git and GitHub for our contributions. We strive to balance three goals with those recommendations:

1. The **volunteer maintainers and contributors** can easily follow the changes you propose, gauge the impact, and come to help you or make a decision.
2. **You as a contributor** can focus most of your time on improving the quality of the nushell project and contributing your expertise to the code or documentation.
3. Making sure we can trace back *why* decisions were made in the past.
This includes discarded approaches. Also we want to quickly identify regressions and fix when something broke.

### How we merge PRs

In general the maintainers **squash** all changes of your PR into a single commit when merging.

This keeps a clean enough linear history, while not forcing you to conform to a too strict style while iterating in your PR or fixing small problems. As an added benefit the commits on the `main` branch are tied to the discussion that happened in the PR through their `#1234` issue number.

> **Note**
> **Pro advice:** In some circumstances, we can agree on rebase-merging a particularly large but connected PR as a series of atomic commits onto the `main` branch to ensure we can more easily revert or bisect particular aspects.

### A good PR makes a change!

As a result of this PR-centric strategy and the general goal that the reviewers should easily understand your change, the **PR title and description matters** a great deal!

Make sure your description is **concise** but contains all relevant information and context.
This means demonstrating what changes, ideally through nushell code or output **examples**.
Furthermore links to technical documentation or instructions for folks that want to play with your change make the review process much easier.

> **Note**
> Try to follow the suggestions in our PR message template to make sure we can quickly focus on the technical merits and impact on the users.

#### A PR should limit itself to a single functional change or related set of same changes.

Mixing different changes in the same PR will make the review process much harder. A PR might get stuck on one aspect while we would actually like to land another change. Furthermore, if we are forced to revert a change, mixing and matching different aspects makes fixing bugs or regressions much harder.

Thus, please try to **separate out unrelated changes**!
**Don't** mix unrelated refactors with a potentially contested change.
Stylistic fixes and housekeeping can be bundled up into singular PRs.

#### Guidelines for the PR title

The PR title should be concise but contain everything for a contributor to know if they should help out in the review of this particular change.

**DON'T**
- `Update file/in/some/deeply/nested/path.rs`
  - Why are you making this change?
- `Fix 2134`
  - What has to be fixed?
  - Hard to follow when not online on GitHub.
- ``Ignore `~` expansion``
  - In what context should this change take effect?
- `[feature] refactor the whole parser and also make nushell indentation-sensitive, upgrade to using Cpython. Let me know what you think!`
  - Be concise
  - Maybe break up into smaller commits or PRs if the title already appears too long?

**DO**
- Mention the nushell feature or command that is affected.
  - ``Fix URL parsing in `http get` (issue #1234)``
- You can mention the issue number if other context is there.
  - In general, mention all related issues in the description to crosslink (e.g. `Fixes #1234`, `Closes #6789`)
- For internal changes mention the area or symbols affected if it helps to clarify
  - ``Factor out `quote_string()` from parser to reuse in `explore` ``

### Review process / Merge conflicts

> **Note**
> Keep in mind that the maintainers are volunteers that need to allocate their attention to several different areas and active PRs. We will try to get back to you as soon as possible.

You can help us to make the review process a smooth experience:
- Testing:
  - We generally review in detail after all the tests pass. Let us know if there is a problem you want to discuss to fix a test failure or forces us to accept a breaking change.
  - If you fix a bug, it is highly recommended that you add a test that reproduces the original issue/panic in a minimal form.
  - In general, added tests help us to understand which assumptions go into a particular addition/change.
  - Try to also test corner cases where those assumptions might break. This can be more valuable than simply adding many similar tests.
- Commit history inside a PR during code review:
  - Good **atomic commits** can help follow larger changes, but we are not pedantic.
  - We don't shame fixup commits while you try to figure out a problem. They can help others see what you tried and what didn't work. (see our [squash policy](#how-we-merge-prs))
  - During active review constant **force pushing** just to amend changes can be confusing!
    - GitHub's UI presents reviewers with less options to compare diffs
    - fetched branches for experimentation become invalid!
    - the notification a maintainer receives has a low signal-to-noise ratio
  - Git pros *can* use their judgement to rebase/squash to clean up the history *if it aids the understanding* of a larger change during review
- Merge conflicts:
  - In general you should take care of resolving merge conflicts.
    - Use your judgement whether to `git merge main` or to `git rebase main`
    - Choose what simplifies having confidence in the conflict resolution and the review. **Merge commits in your branch are OK** in the squash model.
  - Feel free to notify your reviewers or affected PR authors if your change might cause larger conflicts with another change.
  - During the rollup of multiple PRs, we may choose to resolve merge conflicts and CI failures ourselves. (Allow maintainers to push to your branch to enable us to do this quickly.)
 
## Our Rust style
To make the collaboration on a project the scale of Nushell easy, we want to work towards a style of Rust code that can easily be understood by all of our contributors. We conservatively rely on most of [`clippy`s suggestions](https://github.com/rust-lang/rust-clippy) to get to the holy grail of "idiomatic" code. Good code in our eyes is not the most clever use of all available language features or with the most unique personal touch but readable and strikes a balance between being concise, and also unsurprising and explicit in the places where it matters. 
One example of this philosophy is that we generally avoid to fight the borrow-checker in our data model but rather try to get to a correct and simple solution first and then figure out where we should reuse data to achieve the necessary performance. As we are still pre-1.0 this served us well to be able to quickly refactor or change larger parts of the code base. 

### Generally discouraged
#### `+nightly` language features or things only available in the most recent `+stable`
To make life for the people easier that maintain the Nushell packages in various distributions with their own release cycle of `rustc` we typically rely on slightly older Rust versions. We do not make explicit guarantees how far back in the past we live but you can find out in our [`rust-toolchain.toml`](https://github.com/nushell/nushell/blob/main/rust-toolchain.toml)
(As a rule of thumb this has been typically been approximately 2 releases behind the newest stable compiler.)
The use of nightly features is prohibited.

#### Panicking
As Nushell aims to provide a reliable foundational way for folks to interact with their computer, we cannot carelessly crash the execution of their work by panicking Nushell.
Thus panicking is not an allowed error handling strategy for anything that could be triggered by user input OR behavior of the outside system. If Nushell panics this is a bug or we are against all odds already in an unrecoverable state (The system stopped cooperating, we went out of memory). The use of `.unwrap()` is thus outright banned and any uses of `.expect()` or related panicking macros like `unreachable!` should include a helpful description which assumptions have been violated.

#### `unsafe` code
For any use of `unsafe` code we need to require even higher standards and additional review. If you add or alter `unsafe` blocks you have to be familiar with the promises you need to uphold as found in the [Rustonomicon](https://doc.rust-lang.org/nomicon/intro.html). All `unsafe` uses should include `// SAFETY:` comments explaining how the invariants are upheld and thus alerting you what to watch out for when making a change.
##### FFI with system calls and the outside world
As a shell Nushell needs to interact with system APIs in several places, for which FFI code with unsafe blocks may be necessary. In some cases this can be handled by safe API wrapper crates but in some cases we may choose to directly do those calls.
If you do so you need to document the system behavior on top of the Rust memory model guarantees that you uphold. This means documenting whether using a particular system call is safe to use in a particular context and all failure cases are properly recovered.
##### Implementing self-contained data structures
Another motivation for reaching to `unsafe` code might be to try to implement a particular data structure that is not expressable on safe `std` library APIs. Doing so in the Nushell code base would have to clear a high bar for need based on profiling results. Also you should first do a survey of the [crate ecosystem](https://crates.io) that there doesn't exist a usable well vetted crate that already provides safe APIs to the desired datastructure.
##### Make things go faster by removing checks
This is probably a bad idea if you feel tempted to do so. Don't
#### Macros
Another advanced feature people feel tempted to use to work around perceived limitations of Rusts syntax and we are not particularly fans of are custom macros. 
They have clear downsides not only in terms of readability if they locally introduce a different syntax. Most tooling apart from the compiler will struggle more with them. This limits for example consistent automatic formatting or automated refactors with `rust-analyzer`.
That you can fluently read `macro_rules!` is less likely than regular code. This can lead people to introduce funky behavior when using a macro. Be it because a macro is not following proper hygiene rules or because it introduces excessive work at compile time.

So we generally discourage the addition of macros. In a lot of cases your macro may start do something that can be expressed with functions or generics in a much more reusable fashion.
The only exceptions we may allow need to demonstrate that the macro can fix something that is otherwise extremely unreadable, error-prone, or consistently worse at compile time.
### Things we want to get better at
These are things we did pretty liberally to get Nushell off the ground, that make things harder for a high quality stable product. You may run across them but shouldn't take them as an endorsed example.
####  Liberal use of third-party dependencies
The amazing variety of crates on [crates.io](https://crates.io) allowed us to quickly get Nushell into a feature rich state but it left us with a bunch of baggage to clean up.
Each dependency introduces a compile time cost and duplicated code can add to the overall binary size. Also vetting more for correct and secure implementations takes unreasonably more time as this is also a continuous process of reacting to updates or potential vulnerabilities.

Thus we only want to accept dependencies that are essential and well tested implementations of a particular requirement of Nushells codebase.
Also as a project for the move to 1.0 we will try to unify among a set of dependencies if they possibly implement similar things in an area. We don't need three different crates with potentially perfect fit for three problems but rather one reliable crate with a maximized overlap between what it provides and what we need.
We will favor crates that are well tested and used and promise to be more stable and still frequently maintained.
#### Deeply nested code
As  Nushell uses a lot of enums in its internal data representation there are a lot of `match` expressions. Combined with the need to handle a lot of edge cases and be defensive about any errors this has led to some absolutely hard to read deeply nested code (e.g. in the parser but also in the implementation of several commands).
This can be observed both as a "rightward drift" where the main part of the code is found after many levels of indentations or by long function bodies with several layers of branching with seemingly repeated branching inside the higher branch level.
This can also be exacerbated by "quick" bugfixes/enhancements that may just try to add a special case to catch a previously unexpected condition. The likelihood of introducing a bug in a sea of code duplication is high.
To combat this, consider using the early-`return` pattern to reject invalid data early in one place instead of building a tree through Rust's expression constructs with a lot of duplicated paths. Unpacking data into a type that expresses that the necessary things already have been checked and using functions to properly deal with separate and common behavior can also help. 

## License

We use the [MIT License](https://github.com/nushell/nushell/blob/main/LICENSE) in all of our Nushell projects. If you are including or referencing a crate that uses the [GPL License](https://www.gnu.org/licenses/gpl-3.0.en.html#license-text) unfortunately we will not be able to accept your PR.
