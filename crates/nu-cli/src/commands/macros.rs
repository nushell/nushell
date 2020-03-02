#[doc(hidden)]
#[macro_export]
macro_rules! command {
    (
        Named { $export:tt $args:ident $body:block }
        Positional { $($number:tt)* }
        Rest {}
        Signature {
            name: $config_name:tt,
            mandatory_positional: vec![ $($mandatory_positional:tt)* ],
            optional_positional: vec![ $($optional_positional:tt)* ],
            rest_positional: $rest_positional:tt,
            named: {
                $(
                    ($named_param:tt : $named_type:ty : $named_kind:tt)
                )*
            }
        }

        Function {
            $( ( $param_name:tt : $param_type:tt ) )*
        }

        Extract {
            $($extract:tt)*
        }
    ) => {
        #[allow(non_camel_case_types)]
        pub struct $export;

        impl Command for $export {
            fn run(&self, $args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
                fn command($args: EvaluatedCommandArgs, ( $($param_name),*, ): ( $($param_type),*, )) -> Result<OutputStream, ShellError> {
                    let output = $body;

                    Ok(output.boxed().to_output_stream())
                }

                let $args = $args.evaluate_once(registry)?;
                let tuple = ( $($extract ,)* );
                command( $args, tuple )
            }

            fn name(&self) -> &str {
                stringify!($config_name)
            }

            fn config(&self) -> $nu_parser::registry::Signature {
                $nu_parser::registry::Signature {
                    name: self.name().to_string(),
                    positional: vec![$($mandatory_positional)*],
                    rest_positional: false,
                    is_filter: false,
                    is_sink: false,

                    named: {
                        use $nu_parser::registry::NamedType;

                        #[allow(unused_mut)]
                        let mut named: indexmap::IndexMap<String, NamedType> = indexmap::IndexMap::new();

                        $(
                            named.insert(stringify!($named_param).to_string(), $nu_parser::registry::NamedType::$named_kind);
                        )*

                        named
                    }
                }
            }
        }
    };

    // switch
    (
        Named { $export:tt $args:ident $body:block }
        Positional { $($positional_count:tt)* }
        Rest { -- $param_name:ident : Switch , $($rest:tt)* }
        Signature {
            name: $config_name:tt,
            mandatory_positional: vec![ $($mandatory_positional:tt)* ],
            optional_positional: vec![ $($optional_positional:tt)* ],
            rest_positional: $rest_positional:tt,
            named: {
                $($config_named:tt)*
            }
        }
        Function {
            $($function:tt)*
        }
        Extract {
            $($extract:tt)*
        }
    ) => {
        command!(
            Named { $export $args $body }
            Positional { $($positional_count)* + 1 }
            Rest { $($rest)* }
            Signature {
                name: $config_name,
                mandatory_positional: vec![ $($mandatory_positional)* ],
                optional_positional: vec![ $($optional_positional)* ],
                rest_positional: $rest_positional,
                named: {
                    $($config_named)*
                    ($param_name : Switch : Switch)
                }
            }

            Function {
                $($function)* ($param_name : Switch)
            }

            Extract {
                $($extract)* {
                    use std::convert::TryInto;

                    $args.get(stringify!($param_name)).try_into()?
                }
            }
        );
    };

    // mandatory named arguments
    (
        Named { $export:tt $args:ident $body:block }
        Positional { $($positional_count:tt)* }
        Rest { -- $param_name:ident : $param_kind:ty , $($rest:tt)* }
        Signature {
            name: $config_name:tt,
            mandatory_positional: vec![ $($mandatory_positional:tt)* ],
            optional_positional: vec![ $($optional_positional:tt)* ],
            rest_positional: $rest_positional:tt,
            named: {
                $($config_named:tt)*
            }
        }
        Function {
            $($function:tt)*
        }
        Extract {
            $($extract:tt)*
        }
    ) => {
        command!(
            Named { $export $args $body }
            Positional { $($positional_count)* + 1 }
            Rest { $($rest)* }
            Signature {
                name: $config_name,
                mandatory_positional: vec![ $($mandatory_positional)* ],
                optional_positional: vec![ $($optional_positional)* ],
                rest_positional: $rest_positional,
                named: {
                    $($config_named)*
                    ($param_name : Mandatory(NamedValue::Single))
                }
            }

            Function {
                $($function)* ($param_name : $param_kind)
            }

            Extract {
                $($extract)* {
                    use std::convert::TryInto;

                    $args.get(stringify!($param_name)).try_into()?
                }
            }
        );
    };

    // optional named arguments
    (
        Named { $export:tt $args:ident $body:block }
        Positional { $($positional_count:tt)* }
        Rest { -- $param_name:ident ? : $param_kind:ty , $($rest:tt)* }
        Signature {
            name: $config_name:tt,
            mandatory_positional: vec![ $($mandatory_positional:tt)* ],
            optional_positional: vec![ $($optional_positional:tt)* ],
            rest_positional: $rest_positional:tt,
            named: {
                $($config_named:tt)*
            }
        }
        Function {
            $($function:tt)*
        }
        Extract {
            $($extract:tt)*
        }
    ) => {
        command!(
            Named { $export $args $body }
            Positional { $($positional_count)* + 1 }
            Rest { $($rest)* }
            Signature {
                name: $config_name,
                mandatory_positional: vec![ $($mandatory_positional)* ],
                optional_positional: vec![ $($optional_positional)* ],
                rest_positional: $rest_positional,
                named: {
                    $($config_named)*
                    ($param_name : Optional(NamedValue::Single))
                }
            }

            Function {
                $($function)* ($param_name : $param_kind)
            }

            Extract {
                $($extract)* {
                    use std::convert::TryInto;

                    $args.get(stringify!($param_name)).try_into()?
                }
            }
        );
    };

    // mandatory positional block
    (
        Named { $export:ident $args:ident $body:block }
        Positional { $($positional_count:tt)* }
        Rest { $param_name:ident : Block , $($rest:tt)* }
        Signature {
            name: $config_name:tt,
            mandatory_positional: vec![ $($mandatory_positional:tt)* ],
            optional_positional: vec![ $($optional_positional:tt)* ],
            rest_positional: $rest_positional:tt,
            named: {
                $($config_named:tt)*
            }
        }

        Function {
            $($function:tt)*
        }

        Extract {
            $($extract:tt)*
        }

    ) => {
        command!(
            Named { $export $args $body }
            Positional { $($positional_count)* + 1 }
            Rest { $($rest)* }
            Signature {
                name: $config_name,
                mandatory_positional: vec![ $($mandatory_positional)* $nu_parser::registry::PositionalType::mandatory_block(
                    stringify!($param_name)
                ), ],
                optional_positional: vec![ $($optional_positional)* ],
                rest_positional: $rest_positional,
                named: {
                    $($config_named)*
                }
            }

            Function {
                $($function)* ($param_name : Block)
            }

            Extract {
                $($extract:tt)* {
                    use $crate::data::types::ExtractType;
                    let value = $args.expect_nth($($positional_count)*)?;
                    Block::extract(value)?
                }
            }
        );
    };


    // mandatory positional argument
    (
        Named { $export:ident $args:ident $body:block }
        Positional { $($positional_count:tt)* }
        Rest { $param_name:ident : $param_kind:ty , $($rest:tt)* }
        Signature {
            name: $config_name:tt,
            mandatory_positional: vec![ $($mandatory_positional:tt)* ],
            optional_positional: vec![ $($optional_positional:tt)* ],
            rest_positional: $rest_positional:tt,
            named: {
                $($config_named:tt)*
            }
        }

        Function {
            $($function:tt)*
        }

        Extract {
            $($extract:tt)*
        }

    ) => {
        command!(
            Named { $export $args $body }
            Positional { $($positional_count)* + 1 }
            Rest { $($rest)* }
            Signature {
                name: $config_name,
                mandatory_positional: vec![ $($mandatory_positional)* $nu_parser::registry::PositionalType::mandatory(
                    stringify!($param_name), <$param_kind>::syntax_type()
                ), ],
                optional_positional: vec![ $($optional_positional)* ],
                rest_positional: $rest_positional,
                named: {
                    $($config_named)*
                }
            }

            Function {
                $($function)* ($param_name : $param_kind)
            }

            Extract {
                $($extract:tt)* {
                    use $crate::data::types::ExtractType;
                    let value = $args.expect_nth($($positional_count)*)?;
                    <$param_kind>::extract(&value)?
                }
            }
        );
    };

    ($export:ident as $config_name:tt ( $args:ident , $($command_rest:tt)* ) $body:block) => {
        command!(
            Named { $export $args $body }
            Positional { 0 }
            Rest { $($command_rest)* }
            Signature {
                name: $config_name,
                mandatory_positional: vec![],
                optional_positional: vec![],
                rest_positional: false,
                named: {}
            }

            Function {
            }

            Extract {
            }
        );
    };
}
