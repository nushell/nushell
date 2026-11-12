# nu-version: 0.102.0

module git-completion-utils {
  export const GIT_SKIPABLE_FLAGS = ['-v', '--version', '-h', '--help', '-p', '--paginate', '-P', '--no-pager', '--no-replace-objects', '--bare']

  # Helper function to append token if non-empty
  def append-non-empty [token: string]: list<string> -> list<string> {
    if ($token | is-empty) { $in } else { $in | append $token }
  }

  # Split a string to list of args, taking quotes into account.
  # Code is copied and modified from https://github.com/nushell/nushell/issues/14582#issuecomment-2542596272
  export def args-split []: string -> list<string> {
    # Define our states
    const STATE_NORMAL = 0
    const STATE_IN_SINGLE_QUOTE = 1
    const STATE_IN_DOUBLE_QUOTE = 2
    const STATE_ESCAPE = 3
    const WHITESPACES = [" " "\t" "\n" "\r"]

    # Initialize variables
    mut state = $STATE_NORMAL
    mut current_token = ""
    mut result: list<string> = []
    mut prev_state = $STATE_NORMAL

    # Process each character
    for char in ($in | split chars) {
      if $state == $STATE_ESCAPE {
        # Handle escaped character
        $current_token = $current_token + $char
        $state = $prev_state
      } else if $char == '\' {
        # Enter escape state
        $prev_state = $state
        $state = $STATE_ESCAPE
      } else if $state == $STATE_NORMAL {
        if $char == "'" {
          $state = $STATE_IN_SINGLE_QUOTE
        } else if $char == '"' {
          $state = $STATE_IN_DOUBLE_QUOTE
        } else if ($char in $WHITESPACES) {
          # Whitespace in normal state means token boundary
          $result = $result | append-non-empty $current_token
          $current_token = ""
        } else {
          $current_token = $current_token + $char
        }
      } else if $state == $STATE_IN_SINGLE_QUOTE {
        if $char == "'" {
          $state = $STATE_NORMAL
        } else {
          $current_token = $current_token + $char
        }
      } else if $state == $STATE_IN_DOUBLE_QUOTE {
        if $char == '"' {
          $state = $STATE_NORMAL
        } else {
          $current_token = $current_token + $char
        }
      }
    }
    # Handle the last token
    $result = $result | append-non-empty $current_token
    # Return the result
    $result
  }

  # Get changed files which can be restored by `git checkout --`
  export def get-changed-files []: nothing -> list<string> {
    ^git status -uno --porcelain=2 | lines
    | where $it =~ '^1 [.MD]{2}'
    | each { split row ' ' -n 9 | last }
  }

  # Get files which can be retrieved from a branch/commit by `git checkout <tree-ish>`
  export def get-checkoutable-files []: nothing -> list<string> {
    # Relevant statuses are .M", "MM", "MD", ".D", "UU"
    ^git status -uno --porcelain=2 | lines
    | where $it =~ '^1 ([.MD]{2}|UU)'
    | each { split row ' ' -n 9 | last }
  }

  export def get-all-git-local-refs []: nothing -> list<record<ref: string, obj: string, upstream: string, subject: string>> {
    ^git for-each-ref --format '%(refname:lstrip=2)%09%(objectname:short)%09%(upstream:remotename)%(upstream:track)%09%(contents:subject)' refs/heads | lines | parse "{ref}\t{obj}\t{upstream}\t{subject}"
  }

  export def get-all-git-remote-refs []: nothing -> list<record<ref: string, obj: string, subject: string>> {
    ^git for-each-ref --format '%(refname:lstrip=2)%09%(objectname:short)%09%(contents:subject)' refs/remotes | lines | parse "{ref}\t{obj}\t{subject}"
  }

  # Get local branches, remote branches which can be passed to `git merge`
  export def get-mergable-sources []: nothing -> list<record<value: string, description: string>> {
    let local = get-all-git-local-refs | each {|x| {value: $x.ref description: $'Branch, Local, ($x.obj) ($x.subject), (if ($x.upstream | is-not-empty) { $x.upstream } else { "no upstream" } )'} } | insert style 'light_blue'
    let remote = get-all-git-remote-refs | each {|x| {value: $x.ref description: $'Branch, Remote, ($x.obj) ($x.subject)'} } | insert style 'blue_italic'
    $local | append $remote
  }
}

def "nu-complete git available upstream" [] {
  ^git for-each-ref --format '%(refname:short)' refs/heads refs/remotes | lines
}

def "nu-complete git remotes" [] {
  ^git remote --verbose
  | parse --regex '(?<value>\S+)\s+(?<description>\S+)'
  | uniq-by value # Deduplicate where fetch and push remotes are the same
}

def "nu-complete git log" [] {
  ^git log --pretty=%h | lines | each { |line| $line | str trim }
}

# Yield all existing commits in descending chronological order.
def "nu-complete git commits all" [] {
  ^git rev-list --all --remotes --pretty=oneline | lines | parse "{value} {description}"
}

# Yield commits of current branch only. This is useful for e.g. cut points in
# `git rebase`.
def "nu-complete git commits current branch" [] {
  ^git log --pretty="%h %s" | lines | parse "{value} {description}"
}

# Yield local branches like `main`, `feature/typo_fix`
def "nu-complete git local branches" [] {
  ^git for-each-ref --format '%(refname:short)' refs/heads | lines
}

# Yield remote branches like `origin/main`, `upstream/feature-a`
def "nu-complete git remote branches with prefix" [] {
  ^git for-each-ref --format='%(refname:lstrip=2)' refs/remotes | lines
}

# Yield local and remote branch names which can be passed to `git merge`
def "nu-complete git mergable sources" [] {
  use git-completion-utils *
  let branches = get-mergable-sources
  {
    options: {
        case_sensitive: false,
        completion_algorithm: prefix,
        sort: false,
    },
    completions: $branches
  }
}

def "nu-complete git switch" [] {
  use git-completion-utils *
  let branches = get-mergable-sources
  {
    options: {
        case_sensitive: false,
        completion_algorithm: prefix,
        sort: false,
    },
    completions: $branches
  }
}

def "nu-complete git checkout" [context: string, position?:int] {
  use git-completion-utils *
  let preceding = $context | str substring ..$position
  # See what user typed before, like 'git checkout a-branch a-path'.
  # We exclude some flags from previous tokens, to detect if  a branch name has been used as the first argument.
  # FIXME: This method is still naive, though.
  let prev_tokens = $preceding | str trim | args-split | where ($it not-in $GIT_SKIPABLE_FLAGS)
  # In these scenarios, we suggest only file paths, not branch:
  # - After '--'
  # - First arg is a branch
  # If before '--' is just 'git checkout' (or its alias), we suggest "dirty" files only (user is about to reset file).
  if $prev_tokens.2? == '--' {
    return (get-changed-files)
  }
  if '--' in $prev_tokens {
    return (get-checkoutable-files)
  }
  # Already typed first argument.
  if ($prev_tokens | length) > 2 and $preceding ends-with ' ' {
    # If we are creating a new branch, we may want to specify a start point
    if ("-b" not-in $prev_tokens) and ("-B" not-in $prev_tokens) and ("--orphan" not-in $prev_tokens) {
      return (get-checkoutable-files)
    }
  }
  # The first argument can be local branches, remote branches, files and commits
  # Get local and remote branches
  let branches = get-mergable-sources
  let files = (get-checkoutable-files) | wrap value | insert description 'File' | insert style green
  let commits = ^git rev-list -n 400 --remotes --oneline | lines | split column -n 2 ' ' value description | upsert description {|x| $'Commit, ($x.value) ($x.description)' } | insert style 'light_cyan_dimmed'
  {
    options: {
        case_sensitive: false,
        completion_algorithm: prefix,
        sort: false,
    },
    completions: [...$branches, ...$files, ...$commits]
  }
}

# Arguments to `git rebase --onto <arg1> <arg2>`
def "nu-complete git rebase" [] {
  (nu-complete git local branches)
  | parse "{value}"
  | insert description "local branch"
  | append (nu-complete git remote branches with prefix
            | parse "{value}"
            | insert description "remote branch")
  | append (nu-complete git commits all)
}

def "nu-complete git stash-list" [] {
  git stash list | lines | parse "{value}: {description}"
}

def "nu-complete git tags" [] {
  ^git tag --no-color | lines
}

# See `man git-status` under "Short Format"
# This is incomplete, but should cover the most common cases.
const short_status_descriptions = {
  ".D": "Deleted"
  ".M": "Modified"
  "!" : "Ignored"
  "?" : "Untracked"
  "AU": "Staged, not merged"
  "MD": "Some modifications staged, file deleted in work tree"
  "MM": "Some modifications staged, some modifications untracked"
  "R.": "Renamed"
  "UU": "Both modified (in merge conflict)"
}

def "nu-complete git files" [path?: string] {
  let relevant_statuses = ["?",".M", "MM", "MD", ".D", "UU"]
  ^git status -uall --porcelain=2 (if ($path | describe) == 'string' and ($path | str length) != 0 { $"($path)" } else { "." } )
  | lines
  | first 300             # limit the number of data for performance
  | each { |$it|
    if $it starts-with "1 " {
      $it | parse --regex "1 (?P<short_status>\\S+) (?:\\S+\\s?){6} (?P<value>.+)"
    } else if $it starts-with "2 " {
      $it | parse --regex "2 (?P<short_status>\\S+) (?:\\S+\\s?){6} (?P<value>.+)"
    } else if $it starts-with "u " {
      $it | parse --regex "u (?P<short_status>\\S+) (?:\\S+\\s?){8} (?P<value>.+)"
    } else if $it starts-with "? " {
      $it | parse --regex "(?P<short_status>.{1}) (?P<value>.+)"
    } else {
      { short_status: 'unknown', value: $it }
    }
  }
  | flatten
  | where $it.short_status in $relevant_statuses
  | update value { |row| if ($row.value | str contains " ") { $"`($row.value)`" } else { $row.value } }
  | insert "description" { |e| $short_status_descriptions | get $e.short_status}
}

def "nu-complete git built-in-refs" [] {
  [HEAD FETCH_HEAD ORIG_HEAD]
}

def "nu-complete git refs" [] {
  nu-complete git local branches
  | parse "{value}"
  | insert description Branch
  | append (nu-complete git remotes | parse '{value}' | insert description 'Remote branch')
  | append (nu-complete git tags | parse "{value}" | insert description Tag)
  | append (nu-complete git built-in-refs)
}

def "nu-complete git files-or-refs" [] {
  {
    options: { sort: false },
    completions: (
      nu-complete git files | where description == "Modified" | select value description
      | append (nu-complete git local branches | parse '{value}' | insert description 'Local branch')
      | append (nu-complete git built-in-refs | parse '{value}' | insert description 'Built-in Refs')
      | append (nu-complete git tags | parse '{value}' | insert description Tag)
      | append (nu-complete git remotes | get 'value' | parse '{value}' | insert description 'Remote branch')
    )
  }
}

def "nu-complete git aliases" [] {
  ^git config --get-regexp ^alias\.
  | lines
  | parse "alias.{value} {description}"
}

def "nu-complete git subcommands" [] {
  ^git help -a | lines | where $it starts-with "   " | parse -r '\s*(?P<value>[^ ]+) \s*(?P<description>\w.*)'
  | append (nu-complete git aliases)
  | uniq-by value
}

def "nu-complete git add" [] {
  nu-complete git files
}

def "nu-complete git pull rebase" [] {
  ["false","true","merges","interactive"]
}

def "nu-complete git merge strategies" [] {
  ['ort', 'octopus']
}

def "nu-complete git merge strategy options" [] {
  ['ours', 'theirs']
}


# Check out git branches and files
export extern "git checkout" [
  ...targets: string@"nu-complete git checkout"   # name of the branch or files to checkout
  --conflict: string                              # conflict style (merge or diff3)
  --detach(-d)                                    # detach HEAD at named commit
  --force(-f)                                     # force checkout (throw away local modifications)
  --guess                                         # second guess 'git checkout <no-such-branch>' (default)
  --ignore-other-worktrees                        # do not check if another worktree is holding the given ref
  --ignore-skip-worktree-bits                     # do not limit pathspecs to sparse entries only
  --merge(-m)                                     # perform a 3-way merge with the new branch
  --orphan: string                                # new unparented branch
  --ours(-2)                                      # checkout our version for unmerged files
  --overlay                                       # use overlay mode (default)
  --overwrite-ignore                              # update ignored files (default)
  --patch(-p)                                     # select hunks interactively
  --pathspec-from-file: string                    # read pathspec from file
  --progress                                      # force progress reporting
  --quiet(-q)                                     # suppress progress reporting
  --recurse-submodules                            # control recursive updating of submodules
  --theirs(-3)                                    # checkout their version for unmerged files
  --track(-t)                                     # set upstream info for new branch
  -b                                              # create and checkout a new branch
  -B: string                                      # create/reset and checkout a branch
  -l                                              # create reflog for new branch
]

export extern "git reset" [
  ...targets: string@"nu-complete git checkout"      # name of commit, branch, or files to reset to
  --hard                                          # reset HEAD, index and working tree
  --keep                                          # reset HEAD but keep local changes
  --merge                                         # reset HEAD, index and working tree
  --mixed                                         # reset HEAD and index
  --patch(-p)                                     # select hunks interactively
  --quiet(-q)                                     # be quiet, only report errors
  --soft                                          # reset only HEAD
  --pathspec-from-file: string                    # read pathspec from file
  --pathspec-file-nul                             # with --pathspec-from-file, pathspec elements are separated with NUL character
  --no-refresh                                    # skip refreshing the index after reset
  --recurse-submodules: string                    # control recursive updating of submodules
  --no-recurse-submodules                         # don't recurse into submodules
]

# Download objects and refs from another repository
export extern "git fetch" [
  repository?: string@"nu-complete git remotes" # name of the branch to fetch
  --all                                         # Fetch all remotes
  --append(-a)                                  # Append ref names and object names to .git/FETCH_HEAD
  --atomic                                      # Use an atomic transaction to update local refs.
  --depth: int                                  # Limit fetching to n commits from the tip
  --deepen: int                                 # Limit fetching to n commits from the current shallow boundary
  --shallow-since: string                       # Deepen or shorten the history by date
  --shallow-exclude: string                     # Deepen or shorten the history by branch/tag
  --unshallow                                   # Fetch all available history
  --update-shallow                              # Update .git/shallow to accept new refs
  --negotiation-tip: string                     # Specify which commit/glob to report while fetching
  --negotiate-only                              # Do not fetch, only print common ancestors
  --dry-run                                     # Show what would be done
  --write-fetch-head                            # Write fetched refs in FETCH_HEAD (default)
  --no-write-fetch-head                         # Do not write FETCH_HEAD
  --force(-f)                                   # Always update the local branch
  --keep(-k)                                    # Keep downloaded pack
  --multiple                                    # Allow several arguments to be specified
  --auto-maintenance                            # Run 'git maintenance run --auto' at the end (default)
  --no-auto-maintenance                         # Don't run 'git maintenance' at the end
  --auto-gc                                     # Run 'git maintenance run --auto' at the end (default)
  --no-auto-gc                                  # Don't run 'git maintenance' at the end
  --write-commit-graph                          # Write a commit-graph after fetching
  --no-write-commit-graph                       # Don't write a commit-graph after fetching
  --prefetch                                    # Place all refs into the refs/prefetch/ namespace
  --prune(-p)                                   # Remove obsolete remote-tracking references
  --prune-tags(-P)                              # Remove any local tags that do not exist on the remote
  --no-tags(-n)                                 # Disable automatic tag following
  --refmap: string                              # Use this refspec to map the refs to remote-tracking branches
  --tags(-t)                                    # Fetch all tags
  --recurse-submodules: string                  # Fetch new commits of populated submodules (yes/on-demand/no)
  --jobs(-j): int                               # Number of parallel children
  --no-recurse-submodules                       # Disable recursive fetching of submodules
  --set-upstream                                # Add upstream (tracking) reference
  --submodule-prefix: string                    # Prepend to paths printed in informative messages
  --upload-pack: string                         # Non-default path for remote command
  --quiet(-q)                                   # Silence internally used git commands
  --verbose(-v)                                 # Be verbose
  --progress                                    # Report progress on stderr
  --server-option(-o): string                   # Pass options for the server to handle
  --show-forced-updates                         # Check if a branch is force-updated
  --no-show-forced-updates                      # Don't check if a branch is force-updated
  -4                                            # Use IPv4 addresses, ignore IPv6 addresses
  -6                                            # Use IPv6 addresses, ignore IPv4 addresses
]

# Yield local branches and (if remote is specified) remote branches with colon prefix
def "nu-complete git push" [context: string, position: int] {
  use git-completion-utils *
  let preceding = $context | str substring ..$position
  let tokens = $preceding | str trim | args-split | where ($it not-in $GIT_SKIPABLE_FLAGS)

  # Check if we have a remote argument (2nd token, 1st is 'git', 2nd is 'push', 3rd is remote)
  # BUT, args-split might be different depending on how it's called.
  # "git push origin" -> ["git", "push", "origin"]
  # If we have at least 3 tokens, the 3rd one IS likely the remote.
  # We should double check if the 3rd token is actually a remote.
  
  mut remote = ""
  if ($tokens | length) >= 3 {
    $remote = $tokens.2
  }

  let local_branches = (nu-complete git local branches)
  
  if ($remote | is-empty) {
    return $local_branches
  }

  # If we have a remote, find branches for that remote
  # Use plumbing command to get remote branches, excluding HEAD
  let remote_branches = (^git for-each-ref --format='%(refname:lstrip=3)' $'refs/remotes/($remote)' | lines | where $it != 'HEAD')
  
  # Prefix them with :
  let deletion_candidates = ($remote_branches | each { |it| $":($it)" })

  $local_branches | append $deletion_candidates
}

# Push changes
export extern "git push" [
  remote?: string@"nu-complete git remotes",         # the name of the remote
  ...refs: string@"nu-complete git push"             # the branch / refspec
  --all                                              # push all refs
  --atomic                                           # request atomic transaction on remote side
  --delete(-d)                                       # delete refs
  --dry-run(-n)                                      # dry run
  --exec: string                                     # receive pack program
  --follow-tags                                      # push missing but relevant tags
  --force-with-lease                                 # require old value of ref to be at this value
  --force(-f)                                        # force updates
  --ipv4(-4)                                         # use IPv4 addresses only
  --ipv6(-6)                                         # use IPv6 addresses only
  --mirror                                           # mirror all refs
  --no-verify                                        # bypass pre-push hook
  --porcelain                                        # machine-readable output
  --progress                                         # force progress reporting
  --prune                                            # prune locally removed refs
  --push-option(-o): string                          # option to transmit
  --quiet(-q)                                        # be more quiet
  --receive-pack: string                             # receive pack program
  --recurse-submodules: string                       # control recursive pushing of submodules
  --repo: string                                     # repository
  --set-upstream(-u)                                 # set upstream for git pull/status
  --signed: string                                   # GPG sign the push
  --tags                                             # push tags (can't be used with --all or --mirror)
  --thin                                             # use thin pack
  --verbose(-v)                                      # be more verbose
]

# Pull changes
export extern "git pull" [
  remote?: string@"nu-complete git remotes",         # the name of the remote
  ...refs: string@"nu-complete git local branches",  # the branch / refspec
  --rebase(-r): string@"nu-complete git pull rebase",    # rebase current branch on top of upstream after fetching
  --quiet(-q)                                        # suppress output during transfer and merge
  --verbose(-v)                                      # be more verbose
  --commit                                           # perform the merge and commit the result
  --no-commit                                        # perform the merge but do not commit the result
  --edit(-e)                                         # edit the merge commit message
  --no-edit                                          # use the auto-generated merge commit message
  --cleanup: string                                  # specify how to clean up the merge commit message
  --ff                                               # fast-forward if possible
  --no-ff                                            # create a merge commit in all cases
  --gpg-sign(-S)                                     # GPG-sign the resulting merge commit
  --no-gpg-sign                                      # do not GPG-sign the resulting merge commit
  --log: int                                         # include log messages from merged commits
  --no-log                                           # do not include log messages from merged commits
  --signoff                                          # add Signed-off-by trailer
  --no-signoff                                       # do not add Signed-off-by trailer
  --stat(-n)                                         # show a diffstat at the end of the merge
  --no-stat                                          # do not show a diffstat at the end of the merge
  --squash                                           # produce working tree and index state as if a merge happened
  --no-squash                                        # perform the merge and commit the result
  --verify                                           # run pre-merge and commit-msg hooks
  --no-verify                                        # do not run pre-merge and commit-msg hooks
  --strategy(-s): string                             # use the given merge strategy
  --strategy-option(-X): string                      # pass merge strategy-specific option
  --verify-signatures                                # verify the tip commit of the side branch being merged
  --no-verify-signatures                             # do not verify the tip commit of the side branch being merged
  --summary                                          # show a summary of the merge
  --no-summary                                       # do not show a summary of the merge
  --autostash                                        # create a temporary stash entry before the operation
  --no-autostash                                     # do not create a temporary stash entry before the operation
  --allow-unrelated-histories                        # allow merging histories without a common ancestor
  --no-rebase                                        # do not rebase the current branch on top of the upstream branch
  --all                                              # fetch all remotes
  --append(-a)                                       # append fetched refs to existing contents of FETCH_HEAD
  --atomic                                           # use an atomic transaction to update local refs
  --depth: int                                       # limit fetching to the specified number of commits
  --deepen: int                                      # deepen the history by the specified number of commits
  --shallow-since: string                            # deepen or shorten the history since a specified date
  --shallow-exclude: string                          # exclude commits reachable from a specified branch or tag
  --unshallow                                        # convert a shallow repository to a complete one
  --update-shallow                                   # update .git/shallow with new refs
  --tags(-t)                                         # fetch all tags from the remote
  --jobs(-j): int                                    # number of parallel children for fetching
  --set-upstream                                     # add upstream (tracking) reference
  --upload-pack: string                              # specify non-default path for upload-pack on the remote
  --progress                                         # force progress status even if stderr is not a terminal
  --server-option(-o): string                        # transmit the given string to the server
]

# Switch between branches and commits
export extern "git switch" [
  switch?: string@"nu-complete git switch"        # name of branch to switch to
  start_point?: string@"nu-complete git rebase"   # name of the start point
  --create(-c)                                    # create a new branch
  --detach(-d): string@"nu-complete git log"      # switch to a commit in a detached state
  --force-create(-C): string                      # forces creation of new branch, if it exists then the existing branch will be reset to starting point
  --force(-f)                                     # alias for --discard-changes
  --guess                                         # if there is no local branch which matches then name but there is a remote one then this is checked out
  --ignore-other-worktrees                        # switch even if the ref is held by another worktree
  --merge(-m)                                     # attempts to merge changes when switching branches if there are local changes
  --no-guess                                      # do not attempt to match remote branch names
  --no-progress                                   # do not report progress
  --no-recurse-submodules                         # do not update the contents of sub-modules
  --no-track                                      # do not set "upstream" configuration
  --orphan: string                                # create a new orphaned branch
  --progress                                      # report progress status
  --quiet(-q)                                     # suppress feedback messages
  --recurse-submodules                            # update the contents of sub-modules
  --track(-t)                                     # set "upstream" configuration
]

# Find commits yet to be applied to upstream
export extern "git cherry" [
  upstream?: string@"nu-complete git mergable sources"  # Upstream branch to search for equivalent commits. Defaults to the upstream branch of HEAD.
  head?: string@"nu-complete git mergable sources"      # Working branch; defaults to HEAD.
  limit?: string                                        # Do not report commits up to (and including) limit.
  --verbose(-v)                                         # Show the commit subjects next to the SHA1s.
]

# Apply the change introduced by an existing commit
export extern "git cherry-pick" [
  commit?: string@"nu-complete git commits all" # The commit ID to be cherry-picked
  --edit(-e)                                    # Edit the commit message prior to committing
  --no-commit(-n)                               # Apply changes without making any commit
  --signoff(-s)                                 # Add Signed-off-by line to the commit message
  --ff                                          # Fast-forward if possible
  --continue                                    # Continue the operation in progress
  --abort                                       # Cancel the operation
  --skip                                        # Skip the current commit and continue with the rest of the sequence
]

# Rebase the current branch
export extern "git rebase" [
  branch?: string@"nu-complete git rebase"    # name of the branch to rebase onto
  upstream?: string@"nu-complete git rebase"  # upstream branch to compare against
  --continue                                  # restart rebasing process after editing/resolving a conflict
  --abort                                     # abort rebase and reset HEAD to original branch
  --quit                                      # abort rebase but do not reset HEAD
  --interactive(-i)                           # rebase interactively with list of commits in editor
  --onto?: string@"nu-complete git rebase"    # starting point at which to create the new commits
  --root                                      # start rebase from root commit
]

# Merge from a branch
export extern "git merge" [
  # For now, to make it simple, we only complete branches (not commits) and support single-parent case.
  branch?: string@"nu-complete git mergable sources"         # The source branch
  --edit(-e)                                                 # Edit the commit message prior to committing
  --no-edit                                                  # Do not edit commit message
  --no-commit(-n)                                            # Apply changes without making any commit
  --signoff                                                  # Add Signed-off-by line to the commit message
  --ff                                                       # Fast-forward if possible
  --continue                                                 # Continue after resolving a conflict
  --abort                                                    # Abort resolving conflict and go back to original state
  --quit                                                     # Forget about the current merge in progress
  --strategy(-s): string@"nu-complete git merge strategies"  # Merge strategy
  -X: string@"nu-complete git merge strategy options"        # Option for merge strategy
  --verbose(-v)
  --help
]

# List or change branches
export extern "git branch" [
  ...branch: string@"nu-complete git local branches"             # name of branch (or branches) to operate on
  --abbrev                                                       # use short commit hash prefixes
  --edit-description                                             # open editor to edit branch description
  --merged                                                       # list reachable branches
  --no-merged                                                    # list unreachable branches
  --set-upstream-to: string@"nu-complete git available upstream" # set upstream for branch
  --unset-upstream                                               # remote upstream for branch
  --all(-a)                                                      # list both remote and local branches
  --copy(-c)                                                     # copy branch together with config and reflog
  --format                                                       # specify format for listing branches
  --move(-m)                                                     # rename branch
  --points-at                                                    # list branches that point at an object
  --show-current                                                 # print the name of the current branch
  --verbose(-v)                                                  # show commit and upstream for each branch
  --color                                                        # use color in output
  --quiet(-q)                                                    # suppress messages except errors
  --delete(-d)                                                   # delete branch
  -D                                                             # force delete branch
  --list(-l)                                                     # list branches
  --contains: string@"nu-complete git commits all"               # show only branches that contain the specified commit
  --no-contains                                                  # show only branches that don't contain specified commit
  --track(-t)                                                    # when creating a branch, set upstream
]

# List all variables set in config file, along with their values.
export extern "git config list" [
]

# Emits the value of the specified key.
export extern "git config get" [
]

# Set value for one or more config options.
export extern "git config set" [
]

# Unset value for one or more config options.
export extern "git config unset" [
]

# Rename the given section to a new name.
export extern "git config rename-section" [
]

# Remove the given section from the configuration file.
export extern "git config remove-section" [
]

# Opens an editor to modify the specified config file
export extern "git config edit" [
]

# List or change tracked repositories
export extern "git remote" [
  --verbose(-v)                            # Show URL for remotes
]

# Add a new tracked repository
export extern "git remote add" [
]

# Rename a tracked repository
export extern "git remote rename" [
  remote: string@"nu-complete git remotes"             # remote to rename
  new_name: string                                     # new name for remote
]

# Remove a tracked repository
export extern "git remote remove" [
  remote: string@"nu-complete git remotes"             # remote to remove
]

# Get the URL for a tracked repository
export extern "git remote get-url" [
  remote: string@"nu-complete git remotes"             # remote to get URL for
]

# Set the URL for a tracked repository
export extern "git remote set-url" [
  remote: string@"nu-complete git remotes"             # remote to set URL for
  url: string                                          # new URL for remote
]

# Show changes between commits, working tree etc
export extern "git diff" [
  rev1_or_file?: string@"nu-complete git files-or-refs"
  rev2?: string@"nu-complete git refs"
  --cached                                             # show staged changes
  --name-only                                          # only show names of changed files
  --name-status                                        # show changed files and kind of change
  --no-color                                           # disable color output
]

# Commit changes
export extern "git commit" [
  --all(-a)                                           # automatically stage all modified and deleted files
  --amend                                             # amend the previous commit rather than adding a new one
  --message(-m): string                               # specify the commit message rather than opening an editor
  --reuse-message(-C): string                         # reuse the message from a previous commit
  --reedit-message(-c): string                        # reuse and edit message from a commit
  --fixup: string                                     # create a fixup/amend commit
  --squash: string                                    # squash commit for autosquash rebase
  --reset-author                                      # reset author information
  --short                                             # short-format output for dry-run
  --branch                                            # show branch info in short-format
  --porcelain                                         # porcelain-ready format for dry-run
  --long                                              # long-format output for dry-run
  --null(-z)                                          # use NUL instead of LF in output
  --file(-F): string                                  # read commit message from file
  --author: string                                    # override commit author
  --date: string                                      # override author date
  --template(-t): string                              # use commit message template file
  --signoff(-s)                                       # add Signed-off-by trailer
  --no-signoff                                        # do not add Signed-off-by trailer
  --trailer: string                                   # add trailer to commit message
  --no-verify(-n)                                     # bypass pre-commit and commit-msg hooks
  --verify                                            # do not bypass pre-commit and commit-msg hooks
  --allow-empty                                       # allow commit with no changes
  --allow-empty-message                               # allow commit with empty message
  --cleanup: string                                   # cleanup commit message
  --edit(-e)                                          # edit commit message
  --no-edit                                           # do not edit commit message
  --include(-i)                                       # include given paths in commit
  --only(-o)                                          # commit only specified paths
  --pathspec-from-file: string                        # read pathspec from file
  --pathspec-file-nul                                 # use NUL character for pathspec file
  --untracked-files(-u): string                       # show untracked files
  --verbose(-v)                                       # show diff in commit message template
  --quiet(-q)                                         # suppress commit summary
  --dry-run                                           # show paths to be committed without committing
  --status                                            # include git-status output in commit message
  --no-status                                         # do not include git-status output
  --gpg-sign(-S)                                      # GPG-sign commit
  --no-gpg-sign                                       # do not GPG-sign commit
  ...pathspec: string                                 # commit files matching pathspec
]

# List commits
export extern "git log" [
  # Ideally we'd allow completion of revisions here, but that would make completion of filenames not work.
  -U                                                  # show diffs
  --follow                                            # show history beyond renames (single file only)
  --grep: string                                      # show log entries matching supplied regular expression
]

# Show or change the reflog
export extern "git reflog" [
]

# Stage files
export extern "git add" [
  ...file: string@"nu-complete git add"               # file to add
  --all(-A)                                           # add all files
  --dry-run(-n)                                       # don't actually add the file(s), just show if they exist and/or will be ignored
  --edit(-e)                                          # open the diff vs. the index in an editor and let the user edit it
  --force(-f)                                         # allow adding otherwise ignored files
  --interactive(-i)                                   # add modified contents in the working tree interactively to the index
  --patch(-p)                                         # interactively choose hunks to stage
  --verbose(-v)                                       # be verbose
]

# Delete file from the working tree and the index
export extern "git rm" [
  -r                                                   # recursive
  --force(-f)                                          # override the up-to-date check
  --dry-run(-n)                                        # Don't actually remove any file(s)
  --cached                                             # unstage and remove paths only from the index
]

# Show the working tree status
export extern "git status" [
  --verbose(-v)                                       # be verbose
  --short(-s)                                         # show status concisely
  --branch(-b)                                        # show branch information
  --show-stash                                        # show stash information
]

# Stash changes for later
export extern "git stash push" [
  --patch(-p)                                         # interactively choose hunks to stash
]

# Unstash previously stashed changes
export extern "git stash pop" [
  stash?: string@"nu-complete git stash-list"          # stash to pop
  --index(-i)                                          # try to reinstate not only the working tree's changes, but also the index's ones
]

# List stashed changes
export extern "git stash list" [
]

# Show a stashed change
export extern "git stash show" [
  stash?: string@"nu-complete git stash-list"
  -U                                                  # show diff
]

# Drop a stashed change
export extern "git stash drop" [
  stash?: string@"nu-complete git stash-list"
]

# Create a new git repository
export extern "git init" [
  --initial-branch(-b): string                         # initial branch name
]

# List or manipulate tags
export extern "git tag" [
  --delete(-d): string@"nu-complete git tags"         # delete a tag
]

# Prune all unreachable objects
export extern "git prune" [
  --dry-run(-n)                                       # dry run
  --expire: string                                    # expire objects older than
  --progress                                          # show progress
  --verbose(-v)                                       # report all removed objects
]

# Start a binary search to find the commit that introduced a bug
export extern "git bisect start" [
  bad?: string                 # a commit that has the bug
  good?: string                # a commit that doesn't have the bug
]

# Mark the current (or specified) revision as bad
export extern "git bisect bad" [
]

# Mark the current (or specified) revision as good
export extern "git bisect good" [
]

# Skip the current (or specified) revision
export extern "git bisect skip" [
]

# End bisection
export extern "git bisect reset" [
]

# Show help for a git subcommand
export extern "git help" [
  command?: string@"nu-complete git subcommands"       # subcommand to show help for
]

# git worktree
export extern "git worktree" [
  --help(-h)            # display the help message for this command
  ...args
]

# create a new working tree
export extern "git worktree add" [
  path: path            # directory to clone the branch
  branch?: string@"nu-complete git available upstream" # Branch to clone
  --help(-h)            # display the help message for this command
  --force(-f)           # checkout <branch> even if already checked out in other worktree
  -b                    # create a new branch
  -B                    # create or reset a branch
  --detach(-d)          # detach HEAD at named commit
  --checkout            # populate the new working tree
  --lock                # keep the new working tree locked
  --reason              # reason for locking
  --quiet(-q)           # suppress progress reporting
  --track               # set up tracking mode (see git-branch(1))
  --guess-remote        # try to match the new branch name with a remote-tracking branch
  ...args
]

# list details of each worktree
export extern "git worktree list" [
  --help(-h)            # display the help message for this command
  --porcelain           # machine-readable output
  --verbose(-v)         # show extended annotations and reasons, if available
  --expire              # add 'prunable' annotation to worktrees older than <time>
  -z                    # terminate records with a NUL character
  ...args
]

def "nu-complete worktree list" [] {
  ^git worktree list | to text | parse --regex '(?P<value>\S+)\s+(?P<commit>\w+)\s+(?P<description>\S.*)'
}

# prevent a working tree from being pruned
export extern "git worktree lock" [
  worktree: string@"nu-complete worktree list"
  --reason: string      # reason because the tree is locked
  --help(-h)            # display the help message for this command
  --reason              # reason for locking
  ...args
]

# move a working tree to a new location
export extern "git worktree move" [
  --help(-h)            # display the help message for this command
  --force(-f)           # force move even if worktree is dirty or locked
  ...args
]

# prune working tree information
export extern "git worktree prune" [
  --help(-h)            # display the help message for this command
  --dry-run(-n)         # do not remove, show only
  --verbose(-v)         # report pruned working trees
  --expire              # expire working trees older than <time>
  ...args
]

# remove a working tree
export extern "git worktree remove" [
  worktree: string@"nu-complete worktree list"
  --help(-h)            # display the help message for this command
  --force(-f)           # force removal even if worktree is dirty or locked
]

# allow working tree to be pruned, moved or deleted
export extern "git worktree unlock" [
  worktree: string@"nu-complete worktree list"
  ...args
]

# clones a repo
export extern "git clone" [
  --help(-h)                    # display the help message for this command
  --local(-l)                   # cloning from the local machine
  --no-local                    # use the git transport mechanism even if cloning from a local path
  --no-hardlinks                # force git to copy files when cloning from the local machine
  --shared(-s)                  # setup .git/objects/info/alternates to share objects with the source local repo
  --reference: string           # setup .git/objects/info/alternates to share objects with the =<reference> local repo
  --reference-if-able: string   # same as --reference, but skips empty folders
  --dissociate                  # borrow objects from the referenced repo (--reference)
  --quiet(-q)                   # suppress progress reporting
  --verbose(-v)                 # be verbose
  --progress                    # report progress unless --quiet
  --server-option: string       # transmit the =<option> to the server
  --no-checkout(-n)             # no checkout of HEAD
  --reject-shallow              # reject shallow repository as source
  --no-reject-shallow           # do not reject shallow repository as source
  --bare                        # make a bare git repo
  --sparse                      # initialize the sparse-checkout file
  --filter: string              # partial clone using the given =<filter-spec>
  --mirror                      # mirror the source repo
  --origin(-o): string          # use <name> as the name for the remote origin
  --branch(-b): string          # point HEAD to <name> branch
  --upload-pack(-u): string     # use <upload-pack> as the path in the other end when using ssh
  --template: string            # use <template-dir> as the templates directory
  --config(-c): string          # set a <key>=<value> config variable
  --depth: int                  # shallow clone <depth> commits
  --shallow-since: string       # shallow clone commits newer than =<date>
  --shallow-exclude: string     # do not clone commits reachable from <revision> (branch or tag)
  --single-branch               # clone commit history from a single branch
  --no-single-Branch            # do not clone only one branch
  --no-tags                     # do not clone any tags
  --recurse-submodules          # clone the submodules. Also accepts paths
  --shallow-submodules          # shallow clone submodules with depth 1
  --no-shallow-submodules       # do not shallow clone submodules
  --remote-submodules           # submodules are updating using their remote tracking branch
  --no-remote-submodules        # do not track submodules remote
  --separate-git-dir: string    # place the clone at =<git dir> and link it here
  --jobs(-j): int               # number of simultaneous submodules fetch
  ...args
]

# Restores files in working tree or index to previous versions
export extern "git restore" [
  --help(-h)                                    # Display the help message for this command
  --source(-s)                                  # Restore the working tree files with the content from the given tree
  --patch(-p)                                   # Interactively choose hunks to restore
  --worktree(-W)                                # Restore working tree (default if neither --worktree or --staged is used)
  --staged(-S)                                  # Restore index
  --quiet(-q)                                   # Quiet, suppress feedback messages
  --progress                                    # Force progress reporting
  --no-progress                                 # Suppress progress reporting
  --ours                                        # Restore from index using our version for unmerged files
  --theirs                                      # Restore from index using their version for unmerged files
  --merge(-m)                                   # Restore from index and recreate the conflicted merge in unmerged files
  --conflict: string                            # Like --merge but changes the conflict presentation with =<style>
  --ignore-unmerged                             # Restore from index and ignore unmerged entries (unmerged files are left as is)
  --ignore-skip-worktree-bits                   # Ignore sparse checkout patterns and unconditionally restores any files in <pathspec>
  --recurse-submodules                          # Restore the contents of sub-modules in working tree
  --no-recurse-submodules                       # Do not restore the contents of sub-modules in working tree (default)
  --overlay                                     # Do not remove files that don't exist when restoring from tree with --source
  --no-overlay                                  # Remove files that don't exist when restoring from tree with --source (default)
  --pathspec-from-file: string                  # Read pathspec from file
  --pathspec-file-nul                           # Separate pathspec elements with NUL character when reading from file
  ...pathspecs: string@"nu-complete git files"  # Target pathspecs to restore
]

# Print lines matching a pattern
export extern "git grep" [
  --help(-h)                            # Display the help message for this command
  --cached                              # Search blobs registered in the index file instead of worktree
  --untracked                           # Include untracked files in search
  --no-index                            # Similar to `grep -r`, but with additional benefits, such as using pathspec patterns to limit paths; Cannot be used together with --cached or --untracked
  --no-exclude-standard                 # Include ignored files in search (only useful with --untracked)
  --exclude-standard                    # No not include ignored files in search (only useful with --no-index)
  --recurse-submodules                  # Recursively search in each submodule that is active and checked out
  --text(-a)                            # Process binary files as if they were text
  --textconv                            # Honor textconv filter settings
  --no-textconv                         # Do not honor textconv filter settings (default)
  --ignore-case(-i)                     # Ignore case differences between patterns and files
  -I                                    # Don’t match the pattern in binary files
  --max-depth: int                      # Max <depth> to descend down directories for each pathspec. A value of -1 means no limit.
  --recursive(-r)                       # Same as --max-depth=-1
  --no-recursive                        # Same as --max-depth=0
  --word-regexp(-w)                     # Match the pattern only at word boundary
  --invert-match(-v)                    # Select non-matching lines
  -H                                    # Suppress filename in output of matched lines
  --full-name                           # Force relative path to filename from top directory
  --extended-regexp(-E)                 # Use POSIX extended regexp for patterns
  --basic-regexp(-G)                    # Use POSIX basic regexp for patterns (default)
  --perl-regexp(-P)                     # Use Perl-compatible regular expressions for patterns
  --line-number(-n)                     # Prefix the line number to matching lines
  --column                              # Prefix the 1-indexed byte-offset of the first match from the start of the matching line
  --files-with-matches(-l)              # Print filenames of files that contains matches
  --name-only                           # Same as --files-with-matches
  --files-without-match(-L)             # Print filenames of files that do not contain matches
  --null(-z)                            # Use \0 as the delimiter for pathnames in the output, and print them verbatim
  --only-matching(-o)                   # Print only the matched (non-empty) parts of a matching line, with each such part on a separate output line
  --count(-c)                           # Instead of showing every matched line, show the number of lines that match
  --no-color                            # Same as --color=never
  --break                               # Print an empty line between matches from different files.
  --heading                             # Show the filename above the matches in that file instead of at the start of each shown line.
  --show-function(-p)                   # Show the preceding line that contains the function name of the match, unless the matching line is a function name itself.
  --context(-C): int                    # Show <num> leading and trailing lines, and place a line containing -- between contiguous groups of matches.
  --after-context(-A): int              # Show <num> trailing lines, and place a line containing -- between contiguous groups of matches.
  --before-context(-B): int             # Show <num> leading lines, and place a line containing -- between contiguous groups of matches.
  --function-context(-W)                # Show the surrounding text from the previous line containing a function name up to the one before the next function name
  --max-count(-m): int                  # Limit the amount of matches per file. When using the -v or --invert-match option, the search stops after the specified number of non-matches.
  --threads: int                        # Number of grep worker threads to use. Use --help for more information on grep threads.
  -f: string                            # Read patterns from <file>, one per line.
  -e: string                            # Next parameter is the pattern. Multiple patterns are combined by --or.
  --and                                 # Search for lines that match multiple patterns.
  --or                                  # Search for lines that match at least one of multiple patterns. --or is implied between patterns without --and or --not.
  --not                                 # Search for lines that does not match pattern.
  --all-match                           # When giving multiple pattern expressions combined with --or, this flag is specified to limit the match to files that have lines to match all of them.
  --quiet(-q)                           # Do not output matched lines; instead, exit with status 0 when there is a match and with non-zero status when there isn’t.
  ...pathspecs: string                  # Target pathspecs to limit the scope of the search.
]

export extern "git" [
  command?: string@"nu-complete git subcommands"   # Subcommands
  --version(-v)                                    # Prints the Git suite version that the git program came from
  --help(-h)                                       # Prints the synopsis and a list of the most commonly used commands
  --html-path                                      # Print the path, without trailing slash, where Git’s HTML documentation is installed and exit
  --man-path                                       # Print the manpath (see man(1)) for the man pages for this version of Git and exit
  --info-path                                      # Print the path where the Info files documenting this version of Git are installed and exit
  --paginate(-p)                                   # Pipe all output into less (or if set, $env.PAGER) if standard output is a terminal
  --no-pager(-P)                                   # Do not pipe Git output into a pager
  --no-replace-objects                             # Do not use replacement refs to replace Git objects
  --bare                                           # Treat the repository as a bare repository
]
