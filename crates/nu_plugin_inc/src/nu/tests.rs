mod integration {
    use crate::inc::{Action, SemVerAction};
    use crate::Inc;
    use nu_errors::ShellError;
    use nu_plugin::test_helpers::{plugin, CallStub};
    use nu_protocol::{Primitive, UntaggedValue};
    use nu_test_support::value::column_path;
    use nu_value_ext::ValueExt;

    #[test]
    fn picks_up_one_action_flag_only() {
        plugin(&mut Inc::new())
            .args(
                CallStub::new()
                    .with_long_flag("major")
                    .with_long_flag("minor")
                    .create(),
            )
            .setup(|plugin, returned_values| {
                let actual = format!("{}", returned_values.unwrap_err());

                assert!(actual.contains("can only apply one"));
                assert_eq!(plugin.error, Some("can only apply one".to_string()));
            });
    }

    #[test]
    fn picks_up_major_flag() {
        plugin(&mut Inc::new())
            .args(CallStub::new().with_long_flag("major").create())
            .setup(|plugin, _| {
                let sem_version_part = SemVerAction::Major;
                plugin.expect_action(Action::SemVerAction(sem_version_part))
            });
    }

    #[test]
    fn picks_up_minor_flag() {
        plugin(&mut Inc::new())
            .args(CallStub::new().with_long_flag("minor").create())
            .setup(|plugin, _| {
                let sem_version_part = SemVerAction::Minor;
                plugin.expect_action(Action::SemVerAction(sem_version_part))
            });
    }

    #[test]
    fn picks_up_patch_flag() {
        plugin(&mut Inc::new())
            .args(CallStub::new().with_long_flag("patch").create())
            .setup(|plugin, _| {
                let sem_version_part = SemVerAction::Patch;
                plugin.expect_action(Action::SemVerAction(sem_version_part))
            });
    }

    #[test]
    fn picks_up_argument_for_field() -> Result<(), ShellError> {
        plugin(&mut Inc::new())
            .args(CallStub::new().with_parameter("package.version")?.create())
            .setup(|plugin, _| {
                //FIXME: this will need to be updated
                if let Ok(column_path) = column_path("package.version").as_column_path() {
                    let column_path =
                        UntaggedValue::Primitive(Primitive::ColumnPath(column_path.item))
                            .into_value(column_path.tag);
                    plugin.expect_field(column_path)
                }
            });
        Ok(())
    }
    mod sem_ver {
        use crate::Inc;
        use nu_errors::ShellError;
        use nu_plugin::test_helpers::{expect_return_value_at, plugin, CallStub};
        use nu_protocol::TaggedDictBuilder;
        use nu_source::Tag;
        use nu_test_support::value::string;
        use nu_value_ext::get_data;

        fn cargo_sample_record(with_version: &str) -> nu_protocol::Value {
            TaggedDictBuilder::build(Tag::unknown(), |row| {
                row.insert_value("version".to_string(), string(with_version));
            })
        }

        #[test]
        fn major_input_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
            let run = plugin(&mut Inc::new())
                .args(
                    CallStub::new()
                        .with_long_flag("major")
                        .with_parameter("version")?
                        .create(),
                )
                .input(cargo_sample_record("0.1.3"))
                .setup(|_, _| {})
                .test();

            let actual = expect_return_value_at(run, 0);

            assert_eq!(get_data(&actual, "version").borrow(), &string("1.0.0"));
            Ok(())
        }

        #[test]
        fn minor_input_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
            let run = plugin(&mut Inc::new())
                .args(
                    CallStub::new()
                        .with_long_flag("minor")
                        .with_parameter("version")?
                        .create(),
                )
                .input(cargo_sample_record("0.1.3"))
                .setup(|_, _| {})
                .test();

            let actual = expect_return_value_at(run, 0);

            assert_eq!(get_data(&actual, "version").borrow(), &string("0.2.0"));
            Ok(())
        }

        #[test]
        fn patch_input_using_the_field_passed_as_parameter() -> Result<(), ShellError> {
            let run = plugin(&mut Inc::new())
                .args(
                    CallStub::new()
                        .with_long_flag("patch")
                        .with_parameter("version")?
                        .create(),
                )
                .input(cargo_sample_record("0.1.3"))
                .setup(|_, _| {})
                .test();

            let actual = expect_return_value_at(run, 0);

            assert_eq!(get_data(&actual, "version").borrow(), &string("0.1.4"));
            Ok(())
        }
    }
}
