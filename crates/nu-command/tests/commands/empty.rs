use nu_test_support::{nu, pipeline};

#[test]
fn reports_emptiness() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[are_empty];
                     [([[check]; [[]]      ])]
                     [([[check]; [""]      ])]
                     [([[check]; [{}] ])]
            ]
            | get are_empty
            | all? {
              empty? check
            }
        "#
    ));

    assert_eq!(actual.out, "true");
}
