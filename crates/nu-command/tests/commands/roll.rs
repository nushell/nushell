use nu_test_support::prelude::*;

mod rows {
    use super::*;

    fn table() -> Value {
        test_table![
            ["service", "status"];
            ["ruby", "DOWN"],
            ["db", "DOWN"],
            ["nud", "DOWN"],
            ["expected", "HERE"],
        ]
    }

    #[test]
    fn can_roll_down() -> Result {
        let code = "$in | roll down | first | get status";

        test().run_with_data(code, table()).expect_value_eq("HERE")
    }

    #[test]
    fn can_roll_up() -> Result {
        let code = "$in | roll up --by 3 | first | get status";

        test().run_with_data(code, table()).expect_value_eq("HERE")
    }

    #[test]
    fn roll_down_empty_list_is_unchanged() -> Result {
        test().run("[] | roll down").expect_value_eq(test_list![])
    }

    #[test]
    fn roll_up_empty_list_is_unchanged() -> Result {
        test().run("[] | roll up").expect_value_eq(test_list![])
    }

    #[test]
    fn roll_up_empty_list_with_by_is_unchanged() -> Result {
        test()
            .run("[] | roll up --by 3")
            .expect_value_eq(test_list![])
    }
}

mod columns {
    use super::*;

    fn table() -> Value {
        test_table![
            ["commit_author", "origin", "stars"];
            ["Andres", "EC", "amarillito"],
            ["Darren", "US", "black"],
            ["JT", "US", "black"],
            ["Yehuda", "US", "black"],
            ["Jason", "CA", "gold"],
        ]
    }

    #[test]
    fn can_roll_left() -> Result {
        let code = "$in | roll left | columns | str join '-'";

        test()
            .run_with_data(code, table())
            .expect_value_eq("origin-stars-commit_author")
    }

    #[test]
    fn can_roll_right() -> Result {
        let code = "$in | roll right --by 2 | columns | str join '-'";

        test()
            .run_with_data(code, table())
            .expect_value_eq("origin-stars-commit_author")
    }

    struct ThirtyTwo<'a>(usize, &'a str);

    #[test]
    fn can_roll_the_cells_only_keeping_the_header_names() -> Result {
        let expected_value = ThirtyTwo(32, "bit1-bit2-bit3-bit4-bit5-bit6-bit7-bit8");
        let code = "$in | roll right --by 3 --cells-only | columns | str join '-'";

        test()
            .run_with_data(code, bitstring_to_nu_table("00000100"))
            .expect_value_eq(expected_value.1)
    }

    #[test]
    fn roll_left_empty_record_is_unchanged() -> Result {
        test()
            .run("{} | roll left")
            .expect_value_eq(test_record! {})
    }

    #[test]
    fn roll_right_empty_record_is_unchanged() -> Result {
        test()
            .run("{} | roll right")
            .expect_value_eq(test_record! {})
    }

    #[test]
    fn roll_left_empty_record_in_a_list_is_unchanged() -> Result {
        test()
            .run("[{} {}] | roll left --by 2 | to nuon")
            .expect_value_eq("[{}, {}]")
    }

    #[test]
    fn roll_left_skips_empty_row_in_a_table() -> Result {
        test()
            .run("[{a: 1, b: 2} {}] | roll left | to nuon")
            .expect_value_eq("[{b: 2, a: 1}, {}]")
    }

    #[test]
    fn four_in_bitstring_left_shifted_with_three_bits_should_be_32_in_decimal() -> Result {
        let four_bitstring = "00000100";
        let expected_value = ThirtyTwo(32, "00100000");

        assert_eq!(
            shift_three_bits_to_the_left_to_bitstring(four_bitstring)?,
            expected_value.0.to_string()
        );
        Ok(())
    }

    fn shift_three_bits_to_the_left_to_bitstring(bits: &str) -> Result<String> {
        let code = "$in | roll left --by 3 | transpose bit --ignore-titles
            | get bit
            | reverse
            | enumerate
            | each { |it|
                $it.item * (2 ** $it.index)
            }
            | math sum
        ";
        let actual: i64 = test().run_with_data(code, bitstring_to_nu_table(bits))?;
        Ok(actual.to_string())
    }

    fn bitstring_to_nu_table(bits: &str) -> Value {
        let bits: Vec<i64> = bits
            .chars()
            .map(|bit| bit.to_digit(10).expect("bitstring digit") as i64)
            .collect();

        test_table![
            ["bit1", "bit2", "bit3", "bit4", "bit5", "bit6", "bit7", "bit8"];
            [bits[0], bits[1], bits[2], bits[3], bits[4], bits[5], bits[6], bits[7]],
        ]
    }
}
