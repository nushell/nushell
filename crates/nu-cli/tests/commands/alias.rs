#[cfg(test)]
mod tests {
    use nu_test_support::nu;
    use nu_test_support::playground::Playground;

    #[test]
    fn alias_without_args() {
        //https://github.com/nushell/nushell/issues/2416
        let actual = nu!(
            cwd: ".",
            r#"
            alias e [] {^echo hi nushell | to json}
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
                 alias double_echo [b] {echo $b | to json}
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
                alias double_echo [a b] {echo $a $b}
                double_echo 1 2 | to json
            "#
            );

            assert_eq!(actual.out, "[1,2]");
        })
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
        alias -i clash [a] { echo 1.1 2 3 | each { str from -d $a } | range $a } }
        "#
        );

        assert!(actual.err.contains("alias"));
    }

    // #[test]
    // fn alias_with_math_var() {
    //     let actual = nu!(
    //         cwd: ".",
    //         r#"
    //     alias -i echo_math [math] { echo {= 1 + $math}}
    //     echo_math 1 + 1 | to json
    //     "#
    //     );

    //     assert_eq!(actual.out, "3");
    // }
    // #[test]
    // fn alias_with_math_var2() {
    //     let actual = nu!(
    //         cwd: ".",
    //         r#"
    //     alias -i round-plus-one [nums digits math] { echo $nums | each {= $(str from -d $digits | str to-decimal) + $math}}
    //     round-plus-one 3.45 2 1 + 1 | to json
    //     "#
    //     );

    //     assert_eq!(actual.out, "5.45");
    // }

    #[test]
    fn alias_with_true_and_false() {
        //https://github.com/nushell/nushell/issues/2416
        let actual = nu!(
            cwd: ".",
            r#"
        alias is_empty [a] {if $(echo $a | empty?) == $true { echo $true } { echo $false }}
        is_empty ""
        "#
        );
        assert!(actual.out.contains("true"));
    }

    #[test]
    fn alias_without_var_arg_and_var_arg_usage() {
        //https://github.com/nushell/nushell/issues/2416
        let actual = nu!(
            cwd: ".",
            r#"
            alias e [args] {echo $args}
            e 1 2 3 | to json
        "#
        );
        assert!(actual.err.contains("unexpected argument"));
    }

    #[test]
    fn alias_with_var_arg_and_var_arg_usage() {
        //https://github.com/nushell/nushell/issues/2416
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
    fn alias_with_var_arg_and_conflicting_var_arg_usage() {
        //https://github.com/nushell/nushell/issues/2416
        let actual = nu!(
            cwd: ".",
            r#"
            alias e [args...] {sleep $args; kill $args}
            e 1sec 1
        "#
        );
        assert!(actual.err.contains("Contrary types for variable"));
    }

    #[test]
    fn alias_with_var_arg_and_external_cmd() {
        //https://github.com/nushell/nushell/issues/2416
        let actual = nu!(
            cwd: ".",
            r#"
            alias e [args...] {^echo $args | to json}
            e hi mom
        "#
        );
        #[cfg(not(windows))]
        assert_eq!(actual.out, "\"hi mom\\n\"");
        #[cfg(windows)]
        assert_eq!(actual.out, "\"hi mom\\r\\n\"");
    }

    #[test]
    fn alias_sent_env() {
        //https://github.com/nushell/nushell/issues/1835
        let actual = nu!(
            cwd: ".",
            r#"
            alias set-env [name value] { echo $nu.env | insert $name $value | get SHELL | to json }
            set-env SHELL /bin/nu
        "#
        );
        assert_eq!(actual.out, "\"/bin/nu\"");
    }

    #[test]
    #[cfg(not(windows))]
    fn alias_ls() {
        //https://github.com/nushell/nushell/issues/1632
        let actual = nu!(
            cwd: ".",
            r#"
            touch ~/VIRUS.EXE.NOSCAM
            alias l [x] { ls $x }
            l ~ | to json
        "#
        );
        assert!(actual.out.contains("VIRUS.EXE.NOSCAM"));
    }
}
