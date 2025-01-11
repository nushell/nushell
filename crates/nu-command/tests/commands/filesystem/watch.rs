use nu_test_support::{nu, playground::Playground};

#[cfg(windows)]
#[test]
fn watch_test_pwd_per_drive() {
    Playground::setup("watch_test_pwd_per_drive", |dirs, sandbox| {
        sandbox.mkdir("test_folder");
        let _actual = nu!(
            cwd: dirs.test(),
            "
                subst P: /D | touch out
                subst P: test_folder
                cd test_folder
                mkdir P:\\test_folder_on_p

                let pwd = $env.PWD
                let script = \"watch P:test_folder_on_p { |op, path| $\\\"(date now): $($op) - $($path)\\\\n\\\" | save --append \" + $pwd + \"\\\\change.txt } out+err> \" + $pwd + \"\\\\watch.log\"
                echo $script | save -f nu-watch.sh

                mut line =      \"$nuExecutable = 'nu.exe'\\n\"
                $line = $line + \"$nuScript = '\" + $pwd + \"\\\\nu-watch.sh'\\n\"
                $line = $line + \"$logFile = '\" + $pwd + \"\\\\watch.log'\\n\"
                $line = $line + \"$errorFile = '\" + $pwd + \"\\\\watch.err'\\n\"
                $line = $line + \"$job = Start-Job -Name NuWatch -ScriptBlock {\\n\"
                $line = $line + \"    param($nuExe, $script, $log, $err)\\n\"
                $line = $line + \"    Start-Process -FilePath $nuExe `\\n\"
                $line = $line + \"                  -ArgumentList $script `\\n\"
                $line = $line + \"                  -RedirectStandardOutput $log `\\n\"
                $line = $line + \"                  -RedirectStandardError $err `\\n\"
                $line = $line + \"                  -NoNewWindow `\\n\"
                $line = $line + \"} -ArgumentList $nuExecutable, $nuScript, $logFile, $errorFile\\n\"
                $line = $line + \"Write-Output 'Started job with ID: '$($job.Id)\\n\"
                $line = $line + \"dir > '\" + $pwd + \"\\\\test_folder_on_p\\\\test_file_on_p.txt'\\n\"
                $line = $line + \"sleep 1\\n\"
                $line = $line + \"rm '\" + $pwd + \"\\\\test_folder_on_p\\\\test_file_on_p.txt'\\n\"
                $line = $line + \"sleep 1\\n\"
                $line = $line + \"dir > '\" + $pwd + \"\\\\test_folder_on_p\\\\test_file_on_p.txt'\\n\"
                $line = $line + \"sleep 1\\n\"
                $line = $line + \"rm '\" + $pwd + \"\\\\test_folder_on_p\\\\test_file_on_p.txt'\\n\"
                $line = $line + \"sleep 1\\n\"
                $line = $line + \"dir > '\" + $pwd + \"\\\\test_folder_on_p\\\\test_file_on_p.txt'\\n\"
                $line = $line + \"sleep 1\\n\"
                $line = $line + \"rm '\" + $pwd + \"\\\\test_folder_on_p\\\\test_file_on_p.txt'\\n\"
                $line = $line + \"sleep 10\\n\"
                $line = $line + \"Get-Process -Name nu | Stop-Process -Force\\n\"
                $line = $line + \"get-job | Stop-Job\\n\"
                $line = $line + \"get-job | Remove-Job\\n\"
                $line = $line + \"Write-Output 'Stop and remove all job'\\n\"
                echo $line | save -f powershell_background_job.ps1
                powershell -File powershell_background_job.ps1
            "
        );
        let expected_file = dirs.test().join("test_folder\\change.txt");
        assert!(expected_file.exists());
        assert!(_actual.err.is_empty());

        let _actual = nu!(
            cwd: dirs.test(),
            "
                subst P: /D | touch out
            "
        );
    })
}

#[cfg(unix)]
#[test]
fn watch_test_pwd_per_drive() {
    Playground::setup("watch_test_pwd_per_drive", |dirs, sandbox| {
        sandbox.mkdir("test_folder");
        let _actual = nu!(
            cwd: dirs.test(),
            "
                mkdir test_folder
                cd test_folder
                mkdir test_folder_on_x

                let pwd = $env.PWD
                let script = \"watch test_folder_on_x { |op, path| $\\\"(date now): $($op) - $($path)\\\\n\\\" | save --append \" + $pwd + \"/change.txt } out+err> \" + $pwd + \"/watch.nu.log\"
                echo $script | save -f nu-watch.sh

                mut line =      \"#!/bin/bash\\n\"
                $line = $line + \"nuExecutable='nu'\\n\"
                $line = $line + \"nuScript='source \" + $pwd + \"/nu-watch.sh'\\n\"
                $line = $line + \"logFile='\" + $pwd + \"/watch.bash.log'\\n\"
                $line = $line + \"$nuExecutable -c 'source \" + $pwd + \"/nu-watch.sh' > $logFile 2>&1 &\\n\"
                $line = $line + \"bg_pid=$!\\n\"
                $line = $line + \"touch \" + $pwd + \"/test_folder_on_x/test_file_on_x.txt\\n\"
                $line = $line + \"sleep 5\\n\"
                $line = $line + \"rm \" + $pwd + \"/test_folder_on_x/test_file_on_x.txt\\n\"
                $line = $line + \"sleep 5\\n\"
                $line = $line + \"touch \" + $pwd + \"/test_folder_on_x/test_file_on_x.txt\\n\"
                $line = $line + \"sleep 5\\n\"
                $line = $line + \"rm \" + $pwd + \"/test_folder_on_x/test_file_on_x.txt\\n\"
                $line = $line + \"sleep 5\\n\"
                $line = $line + \"kill $bg_pid\\n\"
                echo $line | save -f bash_background_job.sh
                chmod +x bash_background_job.sh
                ./bash_background_job.sh
            "
        );
        let _expected_file = dirs.test().join("test_folder/change.txt");
        assert!(_expected_file.exists());
    });
}
