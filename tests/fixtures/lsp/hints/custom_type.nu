let version = '1.2.3' | into semver
let typed: semver = $version
let copy = $typed
def bump [v: semver]: semver -> semver { $v | semver bump patch }
bump $version
