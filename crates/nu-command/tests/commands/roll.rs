use nu_test_support::nu;

mod rows {
    use super::*;

    const TABLE: &str = r#"[
        [service, status];

        [ruby,      DOWN]
        [db,        DOWN]
        [nud,       DOWN]
        [expected,  HERE]
    ]"#;

    #[test]
    fn can_roll_down() {
        let actual = nu!(format!(
            r#"
                {TABLE}
                | roll down
                | first
                | get status
            "#,
        ));

        assert_eq!(actual.out, "HERE");
    }

    #[test]
    fn can_roll_up() {
        let actual = nu!(format!(
            r#"
                {TABLE}
                | roll up --by 3
                | first
                | get status
            "#
        ));

        assert_eq!(actual.out, "HERE");
    }
}

mod columns {
    use super::*;

    const TABLE: &str = r#"[
        [commit_author, origin,      stars];

        [     "Andres",     EC, amarillito]
        [     "Darren",     US,      black]
        [   "JT",     US,      black]
        [     "Yehuda",     US,      black]
        [      "Jason",     CA,       gold]
    ]"#;

    #[test]
    fn can_roll_left() {
        let actual = nu!(format!(
            r#"
                {TABLE}
                | roll left
                | columns
                | str join "-"
            "#
        ));

        assert_eq!(actual.out, "origin-stars-commit_author");
    }

    #[test]
    fn can_roll_right() {
        let actual = nu!(format!(
            r#"
                {TABLE}
                | roll right --by 2
                | columns
                | str join "-"
            "#
        ));

        assert_eq!(actual.out, "origin-stars-commit_author");
    }

    struct ThirtyTwo<'a>(usize, &'a str);

    #[test]
    fn can_roll_the_cells_only_keeping_the_header_names() {
        let four_bitstring = bitstring_to_nu_row_pipeline("00000100");
        let expected_value = ThirtyTwo(32, "bit1-bit2-bit3-bit4-bit5-bit6-bit7-bit8");

        let actual = nu!(format!(
            "{four_bitstring} | roll right --by 3 --cells-only | columns | str join '-' "
        ));

        assert_eq!(actual.out, expected_value.1);
    }

    #[test]
    fn four_in_bitstring_left_shifted_with_three_bits_should_be_32_in_decimal() {
        let four_bitstring = "00000100";
        let expected_value = ThirtyTwo(32, "00100000");

        assert_eq!(
            shift_three_bits_to_the_left_to_bitstring(four_bitstring),
            expected_value.0.to_string()
        );
    }

    fn shift_three_bits_to_the_left_to_bitstring(bits: &str) -> String {
        // this pipeline takes the bitstring and outputs a nu row literal
        // for example the number 4 in bitstring:
        //
        //  input: 00000100
        //
        // output:
        //  [
        //   [column1, column2, column3, column4, column5, column6, column7, column8];
        //   [      0,       0,       0,       0,       0,       1,       0,       0]
        //  ]
        //
        let bitstring_as_nu_row_pipeline = bitstring_to_nu_row_pipeline(bits);

        // this pipeline takes the nu bitstring row literal, computes it's
        // decimal value.
        let nu_row_literal_bitstring_to_decimal_value_pipeline = r#"transpose bit --ignore-titles
            | get bit
            | reverse
            | enumerate
            | each { |it|
                $it.item * (2 ** $it.index)
            }
            | math sum
        "#;
        println!(
            "{bitstring_as_nu_row_pipeline} | roll left --by 3 | {nu_row_literal_bitstring_to_decimal_value_pipeline}"
        );
        nu!(
            format!("{bitstring_as_nu_row_pipeline} | roll left --by 3 | {nu_row_literal_bitstring_to_decimal_value_pipeline}")
        ).out
    }

    fn bitstring_to_nu_row_pipeline(bits: &str) -> String {
        format!(
            "echo '{}' | {}",
            bits,
            r#"
                    split chars
                    | each { |it| $it | into int }
                    | rotate --ccw
                    | rename bit1 bit2 bit3 bit4 bit5 bit6 bit7 bit8
                "#
        )
    }
}
