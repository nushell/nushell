// use nu_test_support::fs::{file_contents, Stub};

use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[cfg(windows)]
#[test]
fn watch_test_pwd_per_drive_prepare_nu_watch_script() {
    Playground::setup(
        "watch_test_pwd_per_drive_prepare_nu_watch_script",
        |dirs, sandbox| {
            sandbox.mkdir("test_folder");
            let _actual = nu!(
                cwd: dirs.test(),
                r#"
                echo "nu -c 'watch X:test_folder_on_x { |op, path| $\"(date now): $($op) - $($path)\n\" | save --append change.txt }' out+err>> watch.log" | save nu-watch.sh
                open nu-watch.sh
            "#
            );
            assert_eq!(_actual.out, "nu -c 'watch X:test_folder_on_x { |op, path| $\"(date now): $($op) - $($path)\" | save --append change.txt }' out+err>> watch.log");
            assert!(_actual.err.is_empty());
        },
    )
}

#[cfg(windows)]
#[test]
fn watch_test_pwd_per_drive_prepare_powershell_background_job_script() {
    Playground::setup(
        "watch_test_pwd_per_drive_prepare_powershell_background_job_script",
        |dirs, sandbox| {
            sandbox.mkdir("test_folder");
            let _actual = nu!(
                cwd: dirs.test(),
                "
                mut line = '$nuExecutable = \"nu.exe\"\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '$nuScript = \"' + $env.PWD + '\\nu-watch.sh\"\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '$logFile = \"' + $env.PWD + '\\watch.log\"\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '$errorFile = \"' + $env.PWD + '\\watch.err\"\n'
                echo $line | save --append powershell_background_job.ps1
                $line = 'if (!(Test-Path -Path $nuScript)) {\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '    Write-Output \"Nushell script not found: $nuScript\"\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '    exit 1\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '}\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '$job = Start-Job -Name NuWatch -ScriptBlock {\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '    param($nuExe, $script, $log, $err)\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '    Start-Process -FilePath $nuExe `\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '                  -ArgumentList $script `\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '                  -RedirectStandardOutput $log `\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '                  -RedirectStandardError $err `\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '                  -NoNewWindow `\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '                  -Wait\n'
                echo $line | save --append powershell_background_job.ps1
                $line = '} -ArgumentList $nuExecutable, $nuScript, $logFile, $errorFile\n'
                echo $line | save --append powershell_background_job.ps1
                $line = 'Write-Output \"Started job with ID: $($job.Id)\"\n'
                echo $line | save --append powershell_background_job.ps1
                $line = 'dir > \"' + $env.PWD + '\\test_folder_on_x\\test_file_on_x.txt\"\n'
                echo $line | save --append powershell_background_job.ps1
                $line = 'sleep 2\n'
                echo $line | save --append powershell_background_job.ps1
                $line = 'dir > \"' + $env.PWD + '\\test_folder_on_x\\test_file_on_x.txt\"\n'
                echo $line | save --append powershell_background_job.ps1
                $line = 'get-job | Stop-Job\n'
                echo $line | save --append powershell_background_job.ps1
                $line = 'get-job | Remove-Job\n'
                echo $line | save --append powershell_background_job.ps1
                open powershell_background_job.ps1
            "
            );
            eprintln!("StdOut: {}", _actual.out);
            assert!(_actual.err.is_empty());
        },
    )
}
#[cfg(windows)]
#[test]
fn watch_test_pwd_per_drive_verify_log() {
    Playground::setup(
        "watch_test_pwd_per_drive_background_job",
        |dirs, sandbox| {
            sandbox.mkdir("test_folder");
            let _actual = nu!(
                cwd: dirs.test(),
                r#"
                echo "Sun, 5 Jan 2025 09:53:24 -0800 (now): $Write - $E:\\Study\\nushell\\test_folder_on_x\\test.3.txt" | save change.txt
                mut retries = 3
                mut passed = false
                while ($retries > 0) {
                    if (open change.txt | where $it =~ "test.3.txt" | length) > 0 {
                        $passed = true
                        break
                    }

                    $retries = ($retries - 1)
                }
                if ($passed == false) {
                    echo "Test Failed."
                } else {
                    echo "Test Passed."
                }
            "#
            );
            assert_eq!(_actual.out, "Test Passed.");
            assert!(_actual.err.is_empty());
        },
    )
}

#[cfg(windows)]
#[test]
fn watch_test_pwd_per_drive() {
    Playground::setup("watch_test_pwd_per_drive", |dirs, sandbox| {
        sandbox.mkdir("test_folder");
        let _actual = nu!(
            cwd: dirs.test(),
            "
                subst X: /D | touch out
                subst X: test_folder
                cd test_folder
                mkdir X:\\test_folder_on_x
                let pwd = $env.PWD
                let script = \"watch X:test_folder_on_x { |op, path| $\\\"(date now): $($op) - $($path)\\\\n\\\" | save --append \" + $pwd + \"\\\\change.txt } out+err> \" + $pwd + \"\\\\watch.log\"
                echo $script | save -f nu-watch.sh

                mut line = \"$nuExecutable = 'nu.exe'\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"$nuScript = '\" + $pwd + \"\\\\nu-watch.sh'\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"$logFile = '\" + $pwd + \"\\\\watch.log'\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"$errorFile = '\" + $pwd + \"\\\\watch.err'\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"if (!(Test-Path -Path $nuScript)) {\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"    Write-Output 'Nushell script not found:' + $nuScript\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"    exit 1\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"}\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"$job = Start-Job -Name NuWatch -ScriptBlock {\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"    param($nuExe, $script, $log, $err)\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"    Start-Process -FilePath $nuExe `\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"                  -ArgumentList $script `\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"                  -RedirectStandardOutput $log `\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"                  -RedirectStandardError $err `\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"                  -NoNewWindow `\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"} -ArgumentList $nuExecutable, $nuScript, $logFile, $errorFile\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"Write-Output 'Started job with ID: '$($job.Id)\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"dir > '\" + $pwd + \"\\\\test_folder_on_x\\\\test_file_on_x.txt'\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"sleep 3\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"dir > '\" + $pwd + \"\\\\test_folder_on_x\\\\test_file_on_x.txt'\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"sleep 3\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"Get-Process -Name nu | Stop-Process -Force\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"get-job | Stop-Job\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"get-job | Remove-Job\\n\"
                echo $line | save --append powershell_background_job.ps1
                $line = \"Write-Output 'Stop and remove all job'\\n\"
                echo $line | save --append powershell_background_job.ps1
                powershell -File powershell_background_job.ps1
            "
        );
        let expected_file = dirs.test().join("test_folder\\change.txt");
        assert!(expected_file.exists());
        assert!(_actual.err.is_empty());
    })
}
