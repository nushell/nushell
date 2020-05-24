mod integration {
    use crate::strutils::{Action, ReplaceAction};
    use crate::Str;
    use nu_errors::ShellError;
    use nu_plugin::test_helpers::value::{
        column_path, decimal, get_data, int, string, structured_sample_record, table,
        unstructured_sample_record,
    };
    use nu_plugin::test_helpers::{expect_return_value_at, plugin, CallStub};
    use nu_protocol::{Primitive, UntaggedValue};

    #[test]
    fn picks_up_date_time() {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter("to-date-time", string("%d.%m.%Y %H:%M %P %z"))
                    .create(),
            )
            .input(string("5.8.1994 8:00 am +0000"))
            .input(string("6.9.1995 10:00 am +0000"))
            .input(string("5.8.1994 20:00 pm +0000"))
            .input(string("20.4.2020 8:00 am +0000"))
            .setup(|_, _| {})
            .test();
        let ret_vals = run.unwrap();
        for r in ret_vals {
            let r = r
                .as_ref()
                .unwrap()
                .raw_value()
                .unwrap()
                .as_primitive()
                .unwrap();
            match r {
                Primitive::Date(_) => (),
                _ => panic!("failed to convert string to date"),
            }
        }
    }

    #[test]
    fn picks_up_one_action_flag_only() {
        plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_long_flag("upcase")
                    .with_long_flag("downcase")
                    .create(),
            )
            .setup(|plugin, returned_values| {
                let actual = format!("{}", returned_values.unwrap_err());

                assert!(actual.contains("can only apply one"));
                assert_eq!(plugin.error, Some("can only apply one".to_string()));
            });
    }

    #[test]
    fn picks_up_trim_flag() {
        plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("trim").create())
            .setup(|plugin, _| plugin.expect_action(Action::Trim));
    }

    #[test]
    fn picks_up_capitalize_flag() {
        plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("capitalize").create())
            .setup(|plugin, _| plugin.expect_action(Action::Capitalize));
    }

    #[test]
    fn picks_up_downcase_flag() {
        plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("downcase").create())
            .setup(|plugin, _| plugin.expect_action(Action::Downcase));
    }

    #[test]
    fn picks_up_upcase_flag() {
        plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("upcase").create())
            .setup(|plugin, _| plugin.expect_action(Action::Upcase));
    }

    #[test]
    fn picks_up_to_int_flag() {
        plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("to-int").create())
            .setup(|plugin, _| plugin.expect_action(Action::ToInteger));
    }

    #[test]
    fn picks_up_to_float_flag() {
        plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("to-float").create())
            .setup(|plugin, _| plugin.expect_action(Action::ToFloat));
    }

    #[test]
    fn picks_up_arguments_for_replace_flag() {
        let argument = String::from("replace_text");

        plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter("replace", string(&argument))
                    .create(),
            )
            .setup(|plugin, _| {
                let strategy = ReplaceAction::Direct(argument);
                plugin.expect_action(Action::Replace(strategy));
            });
    }

    #[test]
    fn picks_up_arguments_for_find_replace() {
        let search_argument = String::from("kittens");
        let replace_argument = String::from("jotandrehuda");

        plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter(
                        "find-replace",
                        table(&[string(&search_argument), string(&replace_argument)]),
                    )
                    .create(),
            )
            .setup(|plugin, _| {
                let strategy = ReplaceAction::FindAndReplace(search_argument, replace_argument);
                plugin.expect_action(Action::Replace(strategy))
            });
    }

    #[test]
    fn picks_up_argument_for_field() -> Result<(), ShellError> {
        plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_parameter("package.description")?
                    .create(),
            )
            .setup(|plugin, _| {
                //FIXME: this is possibly not correct
                if let Ok(column_path) = column_path(&[string("package"), string("description")]) {
                    plugin.expect_field(column_path)
                }
            });

        Ok(())
    }

    #[test]
    fn substring_errors_if_start_index_is_greater_than_end_index() {
        plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter("substring", string("3,1"))
                    .create(),
            )
            .setup(|plugin, returned_values| {
                let actual = format!("{}", returned_values.unwrap_err());

                assert!(actual.contains("End must be greater than or equal to Start"));
                assert_eq!(
                    plugin.error,
                    Some("End must be greater than or equal to Start".to_string())
                );
            });
    }

    #[test]
    fn upcases_the_input_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_long_flag("upcase")
                    .with_parameter("name")?
                    .create(),
            )
            .input(structured_sample_record("name", "jotandrehuda"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(get_data(actual, "name"), string("JOTANDREHUDA"));
        Ok(())
    }

    #[test]
    fn trims_the_input_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_long_flag("trim")
                    .with_parameter("name")?
                    .create(),
            )
            .input(structured_sample_record("name", "andres   "))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(get_data(actual, "name"), string("andres"));
        Ok(())
    }

    #[test]
    fn capitalizes_the_input_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_long_flag("capitalize")
                    .with_parameter("name")?
                    .create(),
            )
            .input(structured_sample_record("name", "andres"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(get_data(actual, "name"), string("Andres"));
        Ok(())
    }

    #[test]
    fn downcases_the_input_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_long_flag("downcase")
                    .with_parameter("name")?
                    .create(),
            )
            .input(structured_sample_record("name", "JOTANDREHUDA"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(get_data(actual, "name"), string("jotandrehuda"));
        Ok(())
    }

    #[test]
    fn converts_the_input_to_integer_using_the_field_passed_as_parameter() -> Result<(), ShellError>
    {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_long_flag("to-int")
                    .with_parameter("Nu_birthday")?
                    .create(),
            )
            .input(structured_sample_record("Nu_birthday", "10"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(get_data(actual, "Nu_birthday"), int(10));
        Ok(())
    }
    #[test]
    fn converts_the_input_to_float_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_long_flag("to-float")
                    .with_parameter("PI")?
                    .create(),
            )
            .input(structured_sample_record("PI", "3.1415"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(get_data(actual, "PI"), decimal(3.1415));
        Ok(())
    }

    #[test]
    fn replaces_the_input_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_parameter("rustconf")?
                    .with_named_parameter("replace", string("22nd August 2019"))
                    .create(),
            )
            .input(structured_sample_record("rustconf", "1st January 1970"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(get_data(actual, "rustconf"), string("22nd August 2019"));
        Ok(())
    }

    #[test]
    fn find_and_replaces_the_input_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_parameter("staff")?
                    .with_named_parameter(
                        "find-replace",
                        table(&[string("kittens"), string("jotandrehuda")]),
                    )
                    .create(),
            )
            .input(structured_sample_record("staff", "wykittens"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(get_data(actual, "staff"), string("wyjotandrehuda"));
        Ok(())
    }

    #[test]
    fn upcases_the_input() {
        let run = plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("upcase").create())
            .input(unstructured_sample_record("joandrehuda"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);
        assert_eq!(actual, string("JOANDREHUDA"));
    }

    #[test]
    fn trims_the_input() {
        let run = plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("trim").create())
            .input(unstructured_sample_record("andres   "))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);
        assert_eq!(actual, string("andres"));
    }

    #[test]
    fn capitalizes_the_input() {
        let run = plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("capitalize").create())
            .input(unstructured_sample_record("andres"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);
        assert_eq!(actual, string("Andres"));
    }

    #[test]
    fn downcases_the_input() {
        let run = plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("downcase").create())
            .input(unstructured_sample_record("JOANDREHUDA"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);
        assert_eq!(actual, string("joandrehuda"));
    }

    #[test]
    fn converts_the_input_to_integer() {
        let run = plugin(&mut Str::new())
            .args(CallStub::new().with_long_flag("to-int").create())
            .input(unstructured_sample_record("10"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(actual, UntaggedValue::int(10).into_untagged_value());
    }

    #[test]
    fn substrings_the_input() {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter("substring", string("0,1"))
                    .create(),
            )
            .input(unstructured_sample_record("0123456789"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(actual, string("0"));
    }

    #[test]
    fn substrings_the_input_and_returns_the_string_if_end_index_exceeds_length() {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter("substring", string("0,11"))
                    .create(),
            )
            .input(unstructured_sample_record("0123456789"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(actual, string("0123456789"));
    }

    #[test]
    fn substrings_the_input_and_returns_blank_if_start_index_exceeds_length() {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter("substring", string("20,30"))
                    .create(),
            )
            .input(unstructured_sample_record("0123456789"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(actual, string(""));
    }

    #[test]
    fn substrings_the_input_and_treats_start_index_as_zero_if_blank_start_index_given() {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter("substring", string(",5"))
                    .create(),
            )
            .input(unstructured_sample_record("0123456789"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(actual, string("01234"));
    }

    #[test]
    fn substrings_the_input_and_treats_end_index_as_length_if_blank_end_index_given() {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter("substring", string("2,"))
                    .create(),
            )
            .input(unstructured_sample_record("0123456789"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(actual, string("23456789"));
    }

    #[test]
    fn replaces_the_input() {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter("replace", string("22nd August 2019"))
                    .create(),
            )
            .input(unstructured_sample_record("1st January 1970"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(actual, string("22nd August 2019"));
    }

    #[test]
    fn find_and_replaces_the_input() {
        let run = plugin(&mut Str::new())
            .args(
                CallStub::new()
                    .with_named_parameter(
                        "find-replace",
                        table(&[string("kittens"), string("jotandrehuda")]),
                    )
                    .create(),
            )
            .input(unstructured_sample_record("wykittens"))
            .setup(|_, _| {})
            .test();

        let actual = expect_return_value_at(run, 0);

        assert_eq!(actual, string("wyjotandrehuda"));
    }
}
