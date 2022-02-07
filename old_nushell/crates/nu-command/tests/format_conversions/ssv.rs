use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn from_ssv_text_to_table() {
    Playground::setup("filter_from_ssv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "oc_get_svc.txt",
            r#"
                NAME              LABELS                                    SELECTOR                  IP              PORT(S)
                docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
                kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
                kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open oc_get_svc.txt
                | from ssv
                | nth 0
                | get IP
            "#
        ));

        assert_eq!(actual.out, "172.30.78.158");
    })
}

#[test]
fn from_ssv_text_to_table_with_separator_specified() {
    Playground::setup("filter_from_ssv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "oc_get_svc.txt",
            r#"
                NAME              LABELS                                    SELECTOR                  IP              PORT(S)
                docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
                kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
                kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open oc_get_svc.txt
                | from ssv --minimum-spaces 3
                | nth 0
                | get IP
            "#
        ));

        assert_eq!(actual.out, "172.30.78.158");
    })
}

#[test]
fn from_ssv_text_treating_first_line_as_data_with_flag() {
    Playground::setup("filter_from_ssv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "oc_get_svc.txt",
            r#"
                docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
                kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
                kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
            "#,
        )]);

        let aligned_columns = nu!(
        cwd: dirs.test(), pipeline(
            r#"
                open oc_get_svc.txt
                | from ssv --noheaders -a
                | first
                | get Column1
            "#
        ));

        let separator_based = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open oc_get_svc.txt
                | from ssv --noheaders
                | first
                | get Column1
                
            "#
        ));

        assert_eq!(aligned_columns.out, separator_based.out);
        assert_eq!(separator_based.out, "docker-registry");
    })
}
