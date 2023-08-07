# http get https://api.github.com/repos/nushell/nushell/pulls?q=is%3Apr+merged%3A%3E%3D2021-04-20+ | select html_url user.login title body
# http get https://api.github.com/search/issues?q=repo:nushell/nushell+is:pr+is:merged+merged:%3E2021-05-08 | get items | select html_url user.login title body
# Repos to monitor

def query-week-span [] {
    let site_table = [
        [site repo];
        [Nushell nushell]
        [Extension vscode-nushell-lang]
        [Documentation nushell.github.io]
        [Wasm demo]
        [Nu_Scripts nu_scripts]
        [RFCs rfcs]
        [reedline reedline]
        [Nana nana]
        # ] [Jupyter jupyter]
    ]

    let query_prefix = "https://api.github.com/search/issues?q=repo:nushell/"
    let query_date = (seq date --days 7 -r | get 6)
    let per_page = "100"
    let page_num = "1" # need to implement iterating pages
    let colon = "%3A"
    let gt = "%3E"
    let eq = "%3D"
    let amp = "%26"
    let query_suffix = $"+is($colon)pr+is($colon)merged+merged($colon)($gt)($eq)($query_date)&per_page=100&page=1"

    for repo in $site_table {
        let query_string = $"($query_prefix)($repo.repo)($query_suffix)"
        let site_json = (http get -u $env.GITHUB_USERNAME -p $env.GITHUB_PASSWORD $query_string | get items | select html_url user.login title)

        if not ($site_json | all { |it| $it | is-empty }) {
            print $"(char nl)## ($repo.site)(char nl)"

            for user in ($site_json | group-by user_login | transpose user prs) {
                let user_name = $user.user
                let pr_count = ($user.prs | length)

                print -n $"- ($user_name) created "
                for pr in ($user.prs | enumerate) {
                    if $pr_count == ($pr.index + 1) {
                        print -n $"[($pr.item.title)](char lparen)($pr.item.html_url)(char rparen)"
                    } else {
                        print -n $"[($pr.item.title)](char lparen)($pr.item.html_url)(char rparen), and "
                    }
                }

                print ""
            }
        }
    }
}

# 2019-08-23 was the release of 0.2.0, the first public release
let week_num = ((seq date -b '2019-08-23' -n 7 | length) - 1)
print $"# This week in Nushell #($week_num)(char nl)"

if ($env | select GITHUB_USERNAME | is-empty) or ($env | select GITHUB_PASSWORD | is-empty) {
    print 'Please set GITHUB_USERNAME and GITHUB_PASSWORD in $env to use this script'
} else {
    query-week-span
}
