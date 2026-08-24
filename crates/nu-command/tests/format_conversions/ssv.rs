use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn from_ssv_text_to_table(playground: Playground) -> Result {
    playground.file("oc_get_svc.txt", indoc::indoc!{"
        NAME              LABELS                                    SELECTOR                  IP              PORT(S)
        docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
        kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
        kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
    "})?;

    let code = "
        open oc_get_svc.txt
        | from ssv
        | get 0
        | get IP
    ";

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("172.30.78.158")
}

#[test]
fn from_ssv_text_to_table_with_separator_specified(playground: Playground) -> Result {
    playground.file("oc_get_svc.txt", indoc::indoc!{"
        NAME              LABELS                                    SELECTOR                  IP              PORT(S)
        docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
        kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
        kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
    "})?;

    let code = "
        open oc_get_svc.txt
        | from ssv --minimum-spaces 3
        | get 0
        | get IP
    ";

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("172.30.78.158")
}

#[test]
fn from_ssv_text_treating_first_line_as_data_with_flag(playground: Playground) -> Result {
    playground.file("oc_get_svc.txt", indoc::indoc!{"
        docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
        kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
        kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
    "})?;

    let aligned_code = "
        open oc_get_svc.txt
        | from ssv --noheaders -a
        | first
        | get column0
    ";

    let separator_code = "
        open oc_get_svc.txt
        | from ssv --noheaders
        | first
        | get column0
    ";

    test()
        .cwd(playground.path())
        .run(aligned_code)
        .expect_value_eq("docker-registry")?;
    test()
        .cwd(playground.path())
        .run(separator_code)
        .expect_value_eq("docker-registry")
}
