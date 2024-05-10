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
