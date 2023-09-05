use nu_test_support::nu;

#[test]
fn try_succeed() {
    let output = nu!("try { 345 } catch { echo 'hello' }");

    assert!(output.out.contains("345"));
}

#[test]
fn try_catch() {
    let output = nu!("try { foobarbaz } catch { echo 'hello' }");

    assert!(output.out.contains("hello"));
}

#[test]
fn catch_can_access_error() {
    let output = nu!("try { foobarbaz } catch { |err| $err | get raw }");

    assert!(output.err.contains("External command failed"));
}

#[test]
fn catch_can_access_error_as_dollar_in() {
    let output = nu!("try { foobarbaz } catch { $in | get raw }");

    assert!(output.err.contains("External command failed"));
}

#[test]
fn external_failed_should_be_caught() {
    let output = nu!("try { nu --testbin fail; echo 'success' } catch { echo 'fail' }");

    assert!(output.out.contains("fail"));
}

#[test]
fn loop_try_break_should_be_successful() {
    let output =
        nu!("loop { try { print 'successful'; break } catch { print 'failed'; continue } }");

    assert_eq!(output.out, "successful");
}

#[test]
fn loop_catch_break_should_show_failed() {
    let output = nu!("loop {
            try { invalid 1;
            continue; } catch { print 'failed'; break }
        }
        ");

    assert_eq!(output.out, "failed");
}

#[test]
fn loop_try_ignores_continue() {
    let output = nu!("mut total = 0;
        for i in 0..10 {
            try { if ($i mod 2) == 0 {
            continue;}
            $total += 1
        } catch { echo 'failed'; break }
        }
        echo $total
        ");

    assert_eq!(output.out, "5");
}

#[test]
fn loop_try_break_on_command_should_show_successful() {
    let output = nu!("loop { try { ls; break } catch { echo 'failed';continue }}");

    assert!(!output.out.contains("failed"));
}

#[test]
fn catch_block_can_use_error_object() {
    let output = nu!("try {1 / 0} catch {|err| print ($err | get msg)}");
    assert_eq!(output.out, "Division by zero.")
}

#[test]
#[cfg(not(windows))] // windows requires too much effort to replicate permission errors
fn catch_fs_related_errors() {
    use nu_test_support::{
        fs::{files_exist_at, Stub::EmptyFile},
        playground::Playground,
    };

    Playground::setup("ignore_fs_related_errors", |dirs, playground| {
        let file_names = vec!["test1.txt", "test2.txt", "test3.txt"];

        let files = file_names
            .iter()
            .map(|file_name| EmptyFile(file_name))
            .collect();

        playground.mkdir("subdir").with_files(files);

        let test_dir = dirs.test();
        let subdir = test_dir.join("subdir");
        let mut test_dir_permissions = playground.permissions(test_dir);
        let mut subdir_permissions = playground.permissions(&subdir);

        test_dir_permissions.set_readonly(true);
        subdir_permissions.set_readonly(true);
        test_dir_permissions.apply().unwrap();
        subdir_permissions.apply().unwrap();

        let actual = nu!(
            cwd: test_dir,
            "try { rm test*.txt; \"try\" } catch { \"catch\" }"
        );

        assert_eq!(actual.out, "catch");
        assert!(files_exist_at(file_names.clone(), test_dir));

        let actual = nu!(
            cwd: test_dir,
            "try { cp test*.txt subdir/; \"try\" } catch { \"catch\" }"
        );

        assert_eq!(actual.out, "catch");
        assert!(!files_exist_at(file_names.clone(), &subdir));

        let actual = nu!(
            cwd: test_dir,
            "try { mv test*.txt subdir/; \"try\" } catch { \"catch\" }"
        );

        assert_eq!(actual.out, "catch");
        assert!(!files_exist_at(file_names.clone(), &subdir));
    });
}
