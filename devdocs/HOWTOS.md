# Developer how to guides and SOPs

## Adding a dependency

- Check the existing dependencies of Nushell if they already provide the needed functionality
- Choosing the crate: Align the choice of crate with Nushell's goals and be ready to explain why.
    - Trust/Reliability/Existing support: Crates should have an existing userbase to establish trust, repository can't be archived
    - License compatibility: Nushell is MIT-licensed. Any crate you pick needs to be compatible with that. This excludes strong copyleft licenses (e.g. GPL) or use-restricting licenses (e.g. Business Source License)
    - Match to the requirements: Relevant technical considerations if it provides the necessary function in the best possible way and in doing so has no significant negative externalities.
    - Stability: can the crate provide the necessary stability promises that the behavior of Nushell is not affected by version updates.
- (**If existing dependency on this crate exists**:) check if the version you add causes a duplication as two versions have to be compiled to satisfy another version requirement
    - if possible upgrade older dependency specification (if an outside upstream change is easily possible, may be worth contributing to the crate ecosystem)
    - if not check if it would be responsible to use the older version for your requirements
    - else point out the future work necessary to reconcile this and add a duplication
    - (you can check with `cargo tree --duplicates`)
- Git dependencies can not be published as a release, thus are only to be used to to pull in critical new development/fixes on an existing dependencies. (PRs including a new dependency as a git dependency **will be postponed** until a version is released on crates.io!)
- Specify the full semver string of the version you added and tested ("major.minor.patch") this ensures if cargo would downgrade the version, no features or critical bug fixes are missing.
- try to specify the minimal required set of [features](https://doc.rust-lang.org/cargo/reference/features.html)
- **If** the dependencies is used in more than one crate in the workspace add the version specification to the [`workspace.dependencies`](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#inheriting-a-dependency-from-a-workspace) in the root `Cargo.toml` and simply inherit it with `cratename = { workspace = true }`
- (**If adding a dependency referring to a crate inside the nushell workspace**) make sure the addition does not cause a circular dependency structure. Otherwise it can not be published. (We also consider the `dev-dependencies` with our [script to check the crate release order](https://github.com/nushell/nu_scripts/blame/main/make_release/nu_deps.nu) )


## Creating a new crate

- Generally new crates should be part of `crates/` in the nushell monorepo, unless:
    - both Nushell and a crate that gets published outside the monorepo (like `reedline`) depend on a common interface, so it needs to be released before the nushell workspace AND an existing outside crate -> new independent crate in the `nushell` organization
    - it is a fork of a full existing crate that does not require deep integration into the nushell monorepo, maintaining the fork in the `nushell` organization has the benefit of maintaining the commit history and being more accessible to other interested users (e.g. `nu-ansi-term`)
- If you redistribute code that is part of another project, make sure proper attribution is given and license compatibility is given (we are MIT, if your crate use something compatible but different we may need to dual license the new crate)
- Ensure that no circles in the crate graph are created.
- **Currently** use the same version as the other workspace crates
- Use workspace dependencies in the `Cargo.toml` if necessary/possible
- Ensure all necessary `package` fields are specified: https://doc.rust-lang.org/cargo/reference/publishing.html#before-publishing-a-new-crate
- For author attribution: `The Nushell Project authors`
- Include a proper MIT license file with necessary attribution (releasing via several distribution paths requires this)
- (If crate in the monorepo) add the crate to the `workspace.members` table in the root `Cargo.toml` (currently dependabot only finds the crates specified there)
- Let an experienced `nushell/publishing` member review the PR

## Publishing a release

This is handled by members of the `nushell/publishing` team

The current procedure is specified [adjacent to the helper scripts](https://github.com/nushell/nu_scripts/tree/main/make_release)

## How to prepare PRs for release notes

In order to make creating the release notes easier, we have a script which can generate the release notes from PR descriptions. To make sure we don't fall behind, we should aim to make sure the release notes summary for each PR is complete before merging.

Here are the steps to make sure the PR is properly formatted for the release notes script.

1. If the PR is using an old PR template, or the PR author replaced our template, add a "Release notes summary" section.
    * The "Release notes summary" section *must* be a second-level heading.
      ✅ Correct
      These will be detected as release notes summaries by the script.
      ```md
      ## Release notes summary
      ```
      ```
      ## Release notes summary - What our users need to know
      ```
      
      ❌ Incorrect
      These will **not** be detected as release notes summaries by the script.
      ```md
      # Release notes summary
      ```
      ```
      ### Release notes summary - What our users need to know
      ```
      ```
      **Release notes summary**
      ```
    * If the PR is using our old PR template with a "User-Facing Changes", the easiest way to add this is to simply add a "Release notes summary" section underneath:
      ```md
      # User-Facing Changes
      ## Release notes summary
      ```
2. Edit the release notes summary to make any necessary formatting or style changes.
    * Don't be afraid to edit the release notes summaries provided by the PR author; these should be considered a starting point. We want our release notes to meet a certain level of polish, but don't need every contributor to understand the specifics of how we format and style our release notes.
    * **Make sure *every* change is accounted for** in the release notes summary. If a PR deprecates functionality, or an addition also changes how the previous behavior works, *this needs to be noted in the summary!!*
    * TODO explain TODO(release-notes)
    * TODO explain user friendly title / third level heading
    * If a PR has multiple, distinct changes which should each be given their own section in the release notes, make sure each of these changes are under their own third-level heading.
    ✅ Correct
        ```md
        ## Release notes summary - What our users need to know
        
        ### New foo command
        
        This release adds the `foo` command, which lets you do XYZ.
        
        ### `bar` command `--xyz` flag deprecated
        
        Since the new `foo` command does XYZ, the `bar` command no longer needs the `--xyz` flag.
        ```
        
        ❌ Incorrect: These will incorrectly appear as a single change in the release notes.
        ```md
        ## Release notes summary
        This release adds the `foo` command, which lets you do XYZ.
        
        Since the new `foo` command does XYZ, the `bar` command no longer needs the `--xyz` flag.
        ```
        ```md
        ## Release notes summary
        * This release adds the `foo` command, which lets you do XYZ.
        * Since the new `foo` command does XYZ, the `bar` command no longer needs the `--xyz` flag.
        ```
    
    * We should try to follow a somewhat consistent style so our release notes are cohesive. When editing PR release note summaries, you can **reference the [Release notes style guide](#Release-notes-style-guide)**.
3. Select a category for the release notes summary.
   * Each PR should be assigned **exactly one** label to indicate which section of the release notes the summary should be included in. 
     * There are `notes:` labels for each section of the release notes, such as "Additions", "Breaking changes", and "Deprecations".
   * TODO explain notes:mention == Hall of Fame, is default category
4. TODO explain notes:ready and when it's applicable/not applicable

### Release notes style guide

Here are some guidelines on how to write release note summaries. **These aren't firm rules**, but they can help guide us towards easily understandable and cohesive release notes.

* Describe how changes affect *Nushell itself*
    * **Avoid using the term "users"**
      Our PR template says the release notes summary should contain "what our users need to know". This explains what the *scope* of the summary should be, but this shouldn't affect how we *describe* the changes.
      The subject of the summary should be the feature, command, or whatever is being changed. Describing the change in how "users" interact with Nushell unnecessarily complicates the explanation.
         ✅ Good: The summary describes how the `my-command` command has changed in a straight-forward manner.
         > The `my-command` command now has a `--foobar` flag which enables the foobar functionality.
         
         ⚠️ Avoid: By explaining how the change affects users, the actual thing which has changed (`my-command`) is not clear until the latter half of the sentence.
         > Users can now pass the `--foobar` flag to the `my-command` command to enable the foobar functionality.
         
     
* Describe behavior in reference to the release
    * Avoid using the terms "PR" or "pull request"
      TODO
* Use callouts for notes, tips, and warnings
  TODO explain gh callouts vs blog callouts
* Do not use screenshots to represent code or output that can be represented sufficiently informatively as text in a code fence (triple backtick section). Do use a screenshot to demonstrate things that are not otherwise easy to explain (e.g. new behavior in the completion UI, `explore` etc.). Make sure to use a reasonable crop and resolution, as we want to make sure the page loading times are quick.
