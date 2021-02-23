use nu_test_support::{nu, pipeline};

mod rows {
    use super::*;

    fn table() -> String {
        pipeline(
            r#"
            echo [
                [service, status];
            
                [ruby,      DOWN] 
                [db,        DOWN] 
                [nud,       DOWN] 
                [expected,  HERE]
            ]"#,
        )
    }

    #[test]
    fn roll_down_by_default() {
        let actual = nu!(
        cwd: ".",
        format!("{} | {}", table(), pipeline(r#"
            roll
            | first
            | get status
        "#)));

        assert_eq!(actual.out, "HERE");
    }

    #[test]
    fn can_roll_up() {
        let actual = nu!(
        cwd: ".",
        format!("{} | {}", table(), pipeline(r#"
            roll up 3
            | first
            | get status
        "#)));

        assert_eq!(actual.out, "HERE");
    }
}

mod columns {
    use super::*;

    fn table() -> String {
        pipeline(
            r#"
            echo [
                [commit_author, origin,      stars];
            
                [     "Andres",     EC, amarillito] 
                [     "Darren",     US,      black] 
                [   "Jonathan",     US,      black] 
                [     "Yehuda",     US,      black]
                [      "Jason",     CA,       gold]
            ]"#,
        )
    }

    #[test]
    fn roll_left_by_default() {
        let actual = nu!(
        cwd: ".",
        format!("{} | {}", table(), pipeline(r#"
            roll column
            | get
            | str collect "-"
        "#)));

        assert_eq!(actual.out, "origin-stars-commit_author");
    }

    #[test]
    fn can_roll_in_the_opposite_direction() {
        let actual = nu!(
        cwd: ".",
        format!("{} | {}", table(), pipeline(r#"
            roll column 2 --opposite
            | get
            | str collect "-"
        "#)));

        assert_eq!(actual.out, "origin-stars-commit_author");
    }

    struct ThirtieTwo<'a>(usize, &'a str);

    #[test]
    fn can_roll_the_cells_only_keeping_the_header_names() {
        let four_bitstring = bitstring_to_nu_row_pipeline("00000100");
        let expected_value = ThirtieTwo(32, "bit1-bit2-bit3-bit4-bit5-bit6-bit7-bit8");

        let actual = nu!(
            cwd: ".",
            format!("{} | roll column 3 --opposite --cells-only | get | str collect '-' ", four_bitstring)
        );

        assert_eq!(actual.out, expected_value.1);
    }

    #[test]
    fn four_in_bitstring_left_shifted_with_three_bits_should_be_32_in_decimal() {
        let four_bitstring = "00000100";
        let expected_value = ThirtieTwo(32, "00100000");

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
        //   [Column1, Column2, Column3, Column4, Column5, Column6, Column7, Column8];
        //   [      0,       0,       0,       0,       0,       1,       0,       0]
        //  ]
        //
        let bitstring_as_nu_row_pipeline = bitstring_to_nu_row_pipeline(bits);

        // this pipeline takes the nu bitstring row literal, computes it's
        // decimal value.
        let nu_row_literal_bitstring_to_decimal_value_pipeline = pipeline(
            r#"
            pivot bit --ignore-titles
            | get bit
            | reverse
            | each --numbered {
                = $it.item * (2 ** $it.index)
            }
            | math sum
        "#,
        );

        nu!(
            cwd: ".",
            format!("{} | roll column 3 | {}", bitstring_as_nu_row_pipeline, nu_row_literal_bitstring_to_decimal_value_pipeline)
        ).out
    }

    fn bitstring_to_nu_row_pipeline(bits: &str) -> String {
        format!(
            "echo '{}' | {}",
            bits,
            pipeline(
                r#"
            split chars
            | each { str to-int }
            | rotate counter-clockwise _
            | reject _
            | rename bit1 bit2 bit3 bit4 bit5 bit6 bit7 bit8
        "#
            )
        )
    }
}
