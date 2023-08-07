# Readme
# a .mailmap file needs to be in the root of each repo to aggregate users with multiple email addresses
# we could add users if we need to map multiple email addresses to a single user
# and commit the mailmap file to the repo if we wanted to. Format of the mailmap file is at the
# end of the script.
#
# 1. git clone every repo in the list
# 2. setup repos_root_folder to match your system
# 3. setup the proper slash by system (TODO make the slash system agnostic)
# 4. setup the output folder to the path you want it in

# Generate PR Counts for the XX Clubs.
# example usage: get_pr_counts true
# If true is provided as an argument, the script will also generate CSV files for each
# repo with one line per commit, username, email, date in order for you to figure out
# if you need to update the mailmap file so you can merge multiple users into one.
# If false is provided as an argument, the script will summarize the PR counts and
# display a table with the top 50 rows.
# Whether you run in debug_csv mode or not, the output is written to csv files in the
# $repos_root_folder/20k folder
def get_pr_counts [debug_csv: bool, repos_root_folder = '/Users/fdncred/src'] {
    # let repos_root_folder = 'c:\users\dschroeder\source\repos\forks'
    # let repos_root_folder = '/Users/fdncred/src/forks'
    let repos = [[name, folder];
        [nushell, $'($repos_root_folder)(char psep)nushell'],
        [reedline, $'($repos_root_folder)(char psep)reedline'],
        [scripts, $'($repos_root_folder)(char psep)nu_scripts'],
        [vscode, $'($repos_root_folder)(char psep)vscode-nushell-lang'],
        [nana, $'($repos_root_folder)(char psep)nana'],
        [docs, $'($repos_root_folder)(char psep)nushell.github.io']
    ]

    let output_folder = $'($repos_root_folder)(char psep)20k'
    if not ($output_folder | path exists) {
        mkdir $output_folder
    }

    $repos | each {|repo|
        let repo_name = $repo.name
        let repo_folder = $repo.folder

        let output_file = $'($output_folder)(char psep)($repo_name).csv'
        print $"Working on ($repo_name). Saving to ($output_file)."

        cd $repo.folder

        if $debug_csv {
            # This outputs commit, name, email, date for use for adding info to mailmap file
            git log --pretty=%h»¦«%aN»¦«%aE»¦«%aD |
                lines |
                split column "»¦«" commit name email date |
                upsert date {|d| $d.date | into datetime} |
                to csv |
                save -f ($output_file)
        } else {
            git log --pretty=%h»¦«%aN»¦«%aE»¦«%aD |
                lines |
                split column "»¦«" commit name email date |
                upsert date {|d| $d.date | into datetime} |
                group-by name |
                transpose |
                upsert column1 {|c| $c.column1 | length} |
                sort-by column1 |
                rename name commits |
                reverse |
                to csv |
                save -f ($output_file)
        }
    }

    cd $output_folder

    if not $debug_csv {
        let data = (open docs.csv |
            append (open nana.csv) |
            append (open nushell.csv) |
            append (open reedline.csv) |
            append (open scripts.csv) |
            append (open vscode.csv)
        )

        let data_dfr = ($data | dfr into-df)
        $data_dfr |
            dfr group-by name |
            dfr agg [(dfr col commits | dfr sum | dfr as "all_commits")] |
            dfr collect |
            dfr sort-by all_commits |
            dfr reverse |
            dfr into-nu |
            first 50
    }
}

# .mailmap file
# format
# new_name <new_email> old_name <old_email>
# name <email_we_now_want_to_use> some old name <old email address1>
# name <email_we_now_want_to_use> some old name <old email address2>
# name <email_we_now_want_to_use> some old name <old email address3>
# name <email_we_now_want_to_use> some old name <old email address4>
