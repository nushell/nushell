#[cfg(test)]
mod tests {
    use nu_test_support::nu;
    use nu_test_support::playground::Playground;

    #[test]
    fn alias_without_args() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i e [] {^echo hi nushell | to json}
            e
        "#
        );
        #[cfg(not(windows))]
        assert_eq!(actual.out, "\"hi nushell\\n\"");
        #[cfg(windows)]
        assert_eq!(actual.out, "\"hi nushell\\r\\n\"");
    }

    #[test]
    fn alias_args_work() {
        Playground::setup("append_test_2", |dirs, _| {
            let actual = nu!(
                cwd: dirs.root(),
                r#"
                 alias -i double_echo [b] {echo $b | to json}
                 double_echo 1kb
             "#
            );

            assert_eq!(actual.out, "1024");
        })
    }

    #[test]
    fn alias_args_double_echo() {
        Playground::setup("append_test_1", |dirs, _| {
            let actual = nu!(
                cwd: dirs.root(),
                r#"
                alias -i double_echo [a b] {echo $a $b}
                double_echo 1 2 | to json
            "#
            );

            assert_eq!(actual.out, "[1,2]");
        })
    }

    #[test]
    #[cfg(not(windows))]
    fn alias_parses_path_tilde() {
        let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
        alias -i new-cd [dir] { cd $dir }
        new-cd ~
        pwd
        "#
        );

        //If this fails for you, check for any special unicode characters in your ~ path
        assert!(actual.out.chars().filter(|c| c.clone() == '/').count() == 2);
        #[cfg(target_os = "linux")]
        assert!(actual.out.contains("home"));
        #[cfg(target_os = "macos")]
        assert!(actual.out.contains("Users"));
    }

    #[test]
    fn alias_missing_args_work() {
        Playground::setup("append_test_1", |dirs, _| {
            let actual = nu!(
                cwd: dirs.root(),
                r#"
                alias double_echo [a b] {^echo $a $b}
                double_echo bob
            "#
            );

            assert_eq!(actual.out, "bob");
        })
    }

    #[test]
    #[cfg(not(windows))]
    fn alias_all_args_missing() {
        Playground::setup("append_test_1", |dirs, _| {
            let actual = nu!(
                cwd: dirs.root(),
                r#"
                alias double_echo [a b] {^echo $a $b}
                double_echo
            "#
            );

            assert_eq!(actual.out, "");
        })
    }

    #[test]
    #[ignore]
    fn alias_with_in_str_var_right() {
        // Error from binary of main:
        // /home/leo/repos/nushell/nushell(TypeDeduction)> alias -i lw [rust_newbie] {echo 1 2 3 | where "hello_world" in $rust_newbie | to json }
        // /home/leo/repos/nushell/nushell(TypeDeduction)> lw [ big ]
        // error: Type Error
        //   ┌─ shell:1:11
        //   │
        // 1 │ lw [ big ]
        //   │             Expected row or table, found integer
        let actual = nu!(
            cwd: ".",
            r#"
        alias -i lw [newbie] {echo 1 2 3 | where "hello_world" in $newbie | to json}
        lw [hello_world_test_repo]
        "#
        );
        assert_eq!(actual.out, "[1,2,3]");
    }

    #[test]
    fn alias_with_in_str_var_right_mismatch() {
        let actual = nu!(
            cwd: ".",
            r#"
        alias -i lw [rust_newbie] { echo 1 2 3 | where "hello_world" in $rust_newbie | to json }
        lw [ big_brain_programmer ]
        "#
        );
        assert_eq!(actual.out, "");
    }

    #[test]
    fn alias_with_in_err() {
        //in operator only applicable for strings atm
        let actual = nu!(
            cwd: ".",
            r#"
        alias -i lw [p] {echo 1 2 3 | where $p in [1 3 2] | to json}
        lw /root/sys
        "#
        );
        assert!(actual.err.contains("Type"));
    }

    #[test]
    #[ignore]
    fn alias_with_contains() {
        // Output of command in main
        // /home/leo/repos/nushell/nushell(TypeDeduction)> echo 1 2 3 | where 4 in [1 hi 3] | to json
        // [1,3]
        // /home/leo/repos/nushell/nushell(TypeDeduction)> echo 1 2 3 | where 4 in [1 hi 3] | to json
        // [1,3]
        let actual = nu!(
            cwd: ".",
            r#"
        alias -i lw [p] {echo 1 2 3 | where $p in [1 hi 3] | to json}
        lw 1
        "#
        );
        assert_eq!(actual.out, "[1,2,3]");
    }

    #[test]
    #[ignore]
    fn alias_with_contains_and_var_is_right_side() {
        //Output of command in main
        // /home/leo/repos/nushell/nushell(TypeDeduction)> echo 1 2 3 | where 1 in [1 2 hi] | to json
        // [1,2]
        let actual = nu!(
            cwd: ".",
            r#"
        alias -i lw [p] {echo 1 2 3 | where 1 in $p | to json}
        lw [1 2 hi]
        "#
        );
        assert_eq!(actual.out, "[1,2,3]");
    }

    #[test]
    fn error_alias_wrong_shape_shallow() {
        let actual = nu!(
            cwd: ".",
            r#"
        alias -i round-to [num digits] { echo $num | str from -d $digits }
        round-to 3.45 a
        "#
        );

        assert!(actual.err.contains("Type"));
    }

    #[test]
    fn error_alias_wrong_shape_deep_invocation() {
        let actual = nu!(
        cwd: ".",
        r#"
        alias -i round-to [nums digits] { echo $nums | each {= $(str from -d $digits)}}
        round-to 3.45 a
        "#
        );

        assert!(actual.err.contains("Type"));
    }

    #[test]
    fn error_alias_wrong_shape_deep_binary() {
        let actual = nu!(
        cwd: ".",
        r#"
        alias -i round-plus-one [nums digits] { echo $nums | each {= $(str from -d $digits | str to-decimal) + 1}}
        round-plus-one 3.45 a
        "#
        );

        assert!(actual.err.contains("Type"));
    }

    #[test]
    fn error_alias_wrong_shape_deeper_binary() {
        let actual = nu!(
        cwd: ".",
        r#"
        alias -i round-one-more [num digits] { echo $num | str from -d $(= $digits + 1) }
        round-one-more 3.45 a
        "#
        );

        assert!(actual.err.contains("Type"));
    }

    #[test]
    fn error_alias_syntax_shape_clash() {
        let actual = nu!(
            cwd: ".",
            r#"
        alias -i clash [a] { echo 1.1 2 3 | each { str from -d $a } | range $a }
        "#
        );

        assert!(actual.err.contains("Contrary types for variable $a"));
    }

    #[test]
    #[ignore]
    fn alias_with_math_var() {
        let actual = nu!(
            cwd: ".",
            r#"
         alias -i echo_math [math] { echo {= 1 + $math}}
         echo_math 1 + 1 | to json
         "#
        );

        assert_eq!(actual.out, "3");
    }
    #[test]
    #[ignore]
    fn alias_with_math_var2() {
        // Doesn't work also not on main
        // /home/leo/repos/nushell/nushell(TypeDeduction)> alias -i l [nums digits math] {echo $nums | each {= $(str from -d $digits | str to-decimal) + $math}}}
        // /home/leo/repos/nushell/nushell(TypeDeduction)> l 3.45 2 1
        // error: Coercion error
        //   ┌─ shell:1:11
        //   │
        // 1 │ l 3.45 2 1
        //   │             nothing
        //   │
        //   │           decimal
        let actual = nu!(
            cwd: ".",
            r#"
         alias -i round-plus-one [nums digits math] { echo $nums | each {= $(str from -d $digits | str to-decimal) + $math}}
         round-plus-one 3.45 2 1 + 1 | to json
         "#
        );
        assert_eq!(actual.out, "5.45");
    }

    #[test]
    fn alias_with_true_and_false() {
        //https://github.com/nushell/nushell/issues/2416
        let actual = nu!(
            cwd: ".",
            r#"
        alias -i is_empty [a] {if $(echo $a | empty?) == $true { echo $true } { echo $false }}
        is_empty ""
        "#
        );
        assert!(actual.out.contains("true"));
    }

    #[test]
    fn alias_sent_env() {
        //https://github.com/nushell/nushell/issues/1835
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i set-env [name value] { echo $nu.env | insert $name $value | get SHELL | to json }
            set-env SHELL /bin/nu
        "#
        );
        assert_eq!(actual.out, "\"/bin/nu\"");
    }

    #[test]
    #[ignore]
    fn alias_with_math_arg() {
        // Doesn't also work on main
        // /home/leo/repos/nushell/nushell(TypeDeduction)> alias -i lswh [math] {echo 1 2 3 | where $math | to json }
        // /home/leo/repos/nushell/nushell(TypeDeduction)> lswh $it > 2
        // error: Type Error
        //   ┌─ shell:1:13
        //   │
        // 1 │ lswh $it > 2
        //   │               Expected boolean, found block

        // /home/leo/repos/nushell/nushell(TypeDeduction)> lswh {$it > 2}
        // error: Type Error
        //   ┌─ shell:1:15
        //   │
        // 1 │ lswh {$it > 2}
        //   │                 Expected boolean, found block
        let actual = nu!(
            cwd: ".",
            r#"
             alias -i lswh [math] { echo 1 2 3 | where $math | to json }
             lswh $it > 2
         "#
        );
        assert_eq!(actual.out, "3");
    }

    #[test]
    #[cfg(not(windows))]
    fn alias_ls() {
        //https://github.com/nushell/nushell/issues/1632
        let actual = nu!(
            cwd: ".",
            r#"
            touch /tmp/nushell_alias_test
            alias -i l [x] { ls $x }
            l /tmp | to json
        "#
        );
        assert!(actual.out.contains("nushell_alias_test"));
    }

    #[test]
    fn alias_with_var_arg_no_inference() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias e [args...] {echo $args}
            e 1 2 3 | to json
        "#
        );
        assert_eq!(actual.out, "[1,2,3]");
    }

    #[test]
    fn alias_without_var_arg_and_var_arg_usage() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i e [args] {echo $args}
            e 1 2 3 | to json
        "#
        );
        assert!(actual.err.contains("unexpected argument"));
    }

    #[test]
    fn alias_with_var_arg_and_var_arg_usage() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i e [args...] {echo $args}
            e 1 2 3 | to json
        "#
        );
        assert_eq!(actual.out, "[1,2,3]");
    }

    #[test]
    fn alias_with_var_arg_used_as_normal_positional() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i n [args...] {echo [first second third] | nth $args}
            n 1 2 | to json
        "#
        );
        assert!(actual
            .err
            .contains("VarArg used as a normal positional argument"));
    }

    #[test]
    fn alias_with_var_arg_and_conflicting_var_arg_usage() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i e [args...] {sleep 1sec $args; kill 1 $args}
            e 1sec 1
        "#
        );
        assert!(actual.err.contains("Contrary types for variable"));
    }

    #[test]
    fn alias_with_var_arg_and_external_cmd() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i e [args...] {^echo $args | to json}
            e hi mom
        "#
        );
        #[cfg(not(windows))]
        assert_eq!(actual.out, "\"hi mom\\n\"");
        #[cfg(windows)]
        assert_eq!(actual.out, "\"hi mom\\r\\n\"");
    }

    #[test]
    fn alias_with_var_arg_and_params_after_var_arg() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i n [args...] {echo [1 2 3 4 5] | nth 0 $args 4}
            n 1 3 | to json
        "#
        );
        assert!(actual
            .err
            .contains("No arguments are allowed after a VarArg"));
    }

    #[test]
    fn alias_with_var_arg_and_empty_var_arg() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i n [args...] {echo $args}
            n | to json
        "#
        );
        assert_eq!(actual.out, "");
    }

    #[test]
    fn alias_with_var_arg_not_in_last_place() {
        let actual = nu!(
            cwd: ".",
            r#"
            alias -i n [args... arg] {echo $arg $args}
            n | to json
        "#
        );
        assert!(actual
            .err
            .contains("Var-args variables are only allowed as the last argument!"));
    }
}
