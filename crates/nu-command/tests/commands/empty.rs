use nu_test_support::{nu, pipeline};

// FIXME: jt: needs more work
#[ignore]
#[test]
fn reports_emptiness() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[are_empty];
                     [([[check]; [[]]      ])]
                     [([[check]; [""]      ])]
                     [([[check]; [(wrap)] ])]
            ]
            | get are_empty
            | empty? check
            | where check
            | length
        "#
    ));

    assert_eq!(actual.out, "3");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn sets_block_run_value_for_an_empty_column() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [
                     [  first_name, last_name,   rusty_at, likes  ];
                     [      Andrés,  Robalino, 10/11/2013,   1    ]
                     [    Jonathan,    Turner, 10/12/2013,   1    ]
                     [       Jason,     Gedge, 10/11/2013,   1    ]
                     [      Yehuda,      Katz, 10/11/2013,  ''    ]
            ]
            | empty? likes -b {|_| 1 }
            | get likes
            | math sum
        "#
    ));

    assert_eq!(actual.out, "4");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn sets_block_run_value_for_many_empty_columns() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [
                     [  boost   check   ];
                     [     1,    []     ]
                     [     1,    ""     ]
                     [     1,  (wrap)  ]
            ]
            | empty? boost check -b { 1 }
            | get boost check
            | math sum
        "#
    ));

    assert_eq!(actual.out, "6");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn passing_a_block_will_set_contents_on_empty_cells_and_leave_non_empty_ones_untouched() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [
                     [      NAME, LVL,   HP ];
                     [    Andrés,  30, 3000 ]
                     [  Alistair,  29, 2900 ]
                     [    Arepas,  "",   "" ]
                     [     Jorge,  30, 3000 ]
            ]
            | empty? LVL -b { 9 }
            | empty? HP -b {
                $it.LVL * 1000
              }
            | math sum
            | get HP
        "#
    ));

    assert_eq!(actual.out, "17900");
}
