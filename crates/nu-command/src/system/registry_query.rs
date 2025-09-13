use nu_engine::command_prelude::*;

use nu_protocol::shell_error::io::IoError;
use windows::{Win32::System::Environment::ExpandEnvironmentStringsW, core::PCWSTR};
use winreg::{RegKey, enums::*, types::FromRegValue};

#[derive(Clone)]
pub struct RegistryQuery;

impl Command for RegistryQuery {
    fn name(&self) -> &str {
        "registry query"
    }

    fn signature(&self) -> Signature {
        Signature::build("registry query")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .switch("hkcr", "query the hkey_classes_root hive", None)
            .switch("hkcu", "query the hkey_current_user hive", None)
            .switch("hklm", "query the hkey_local_machine hive", None)
            .switch("hku", "query the hkey_users hive", None)
            .switch("hkpd", "query the hkey_performance_data hive", None)
            .switch("hkpt", "query the hkey_performance_text hive", None)
            .switch("hkpnls", "query the hkey_performance_nls_text hive", None)
            .switch("hkcc", "query the hkey_current_config hive", None)
            .switch("hkdd", "query the hkey_dyn_data hive", None)
            .switch(
                "hkculs",
                "query the hkey_current_user_local_settings hive",
                None,
            )
            .switch(
                "no-expand",
                "do not expand %ENV% placeholders in REG_EXPAND_SZ",
                Some('u'),
            )
            .required("key", SyntaxShape::String, "Registry key to query.")
            .optional(
                "value",
                SyntaxShape::String,
                "Optionally supply a registry value to query.",
            )
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "Query the Windows registry."
    }

    fn extra_description(&self) -> &str {
        "Currently supported only on Windows systems."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        registry_query(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Query the HKEY_CURRENT_USER hive",
                example: "registry query --hkcu environment",
                result: None,
            },
            Example {
                description: "Query the HKEY_LOCAL_MACHINE hive",
                example: r"registry query --hklm 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'",
                result: None,
            },
        ]
    }
}

fn registry_query(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let call_span = call.head;

    let skip_expand = call.has_flag(engine_state, stack, "no-expand")?;

    let registry_key: Spanned<String> = call.req(engine_state, stack, 0)?;
    let registry_key_span = &registry_key.clone().span;
    let registry_value: Option<Spanned<String>> = call.opt(engine_state, stack, 1)?;

    let reg_hive = get_reg_hive(engine_state, stack, call)?;
    let reg_key = reg_hive
        .open_subkey(registry_key.item)
        .map_err(|err| IoError::new(err, *registry_key_span, None))?;

    if registry_value.is_none() {
        let mut reg_values = vec![];
        for (name, val) in reg_key.enum_values().flatten() {
            let reg_type = format!("{:?}", val.vtype);
            let nu_value = reg_value_to_nu_value(val, call_span, skip_expand);
            reg_values.push(Value::record(
                record! {
                    "name" => Value::string(name, call_span),
                    "value" => nu_value,
                    "type" => Value::string(reg_type, call_span),
                },
                *registry_key_span,
            ))
        }
        Ok(reg_values.into_pipeline_data(call_span, engine_state.signals().clone()))
    } else {
        match registry_value {
            Some(value) => {
                let reg_value = reg_key.get_raw_value(value.item.as_str());
                match reg_value {
                    Ok(val) => {
                        let reg_type = format!("{:?}", val.vtype);
                        let nu_value = reg_value_to_nu_value(val, call_span, skip_expand);
                        Ok(Value::record(
                            record! {
                                "name" => Value::string(value.item, call_span),
                                "value" => nu_value,
                                "type" => Value::string(reg_type, call_span),
                            },
                            value.span,
                        )
                        .into_pipeline_data())
                    }
                    Err(_) => Err(ShellError::GenericError {
                        error: "Unable to find registry key/value".into(),
                        msg: format!("Registry value: {} was not found", value.item),
                        span: Some(value.span),
                        help: None,
                        inner: vec![],
                    }),
                }
            }
            None => Ok(Value::nothing(call_span).into_pipeline_data()),
        }
    }
}

fn get_reg_hive(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<RegKey, ShellError> {
    let flags = [
        "hkcr", "hkcu", "hklm", "hku", "hkpd", "hkpt", "hkpnls", "hkcc", "hkdd", "hkculs",
    ]
    .iter()
    .copied()
    .filter_map(|flag| match call.has_flag(engine_state, stack, flag) {
        Ok(true) => Some(Ok(flag)),
        Ok(false) => None,
        Err(e) => Some(Err(e)),
    })
    .collect::<Result<Vec<_>, ShellError>>()?;
    if flags.len() > 1 {
        return Err(ShellError::GenericError {
            error: "Only one registry key can be specified".into(),
            msg: "Only one registry key can be specified".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        });
    }
    let hive = flags.first().copied().unwrap_or("hkcu");
    let hkey = match hive {
        "hkcr" => HKEY_CLASSES_ROOT,
        "hkcu" => HKEY_CURRENT_USER,
        "hklm" => HKEY_LOCAL_MACHINE,
        "hku" => HKEY_USERS,
        "hkpd" => HKEY_PERFORMANCE_DATA,
        "hkpt" => HKEY_PERFORMANCE_TEXT,
        "hkpnls" => HKEY_PERFORMANCE_NLSTEXT,
        "hkcc" => HKEY_CURRENT_CONFIG,
        "hkdd" => HKEY_DYN_DATA,
        "hkculs" => HKEY_CURRENT_USER_LOCAL_SETTINGS,
        _ => {
            return Err(ShellError::NushellFailedSpanned {
                msg: "Entered unreachable code".into(),
                label: "Unknown registry hive".into(),
                span: call.head,
            });
        }
    };
    Ok(RegKey::predef(hkey))
}

fn reg_value_to_nu_value(
    mut reg_value: winreg::RegValue,
    call_span: Span,
    skip_expand: bool,
) -> nu_protocol::Value {
    match reg_value.vtype {
        REG_NONE => Value::nothing(call_span),
        REG_BINARY => Value::binary(reg_value.bytes, call_span),
        REG_MULTI_SZ => reg_value_to_nu_list_string(reg_value, call_span),
        REG_SZ | REG_EXPAND_SZ => reg_value_to_nu_string(reg_value, call_span, skip_expand),
        REG_DWORD | REG_DWORD_BIG_ENDIAN | REG_QWORD => reg_value_to_nu_int(reg_value, call_span),

        // This should be impossible, as registry symlinks should be automatically transparent
        // to the registry API as it's used by winreg, since it never uses REG_OPTION_OPEN_LINK.
        // If it happens, decode as if the link is a string; it should be a registry path string.
        REG_LINK => {
            reg_value.vtype = REG_SZ;
            reg_value_to_nu_string(reg_value, call_span, skip_expand)
        }

        // Decode these as binary; that seems to be the least bad option available to us.
        // REG_RESOURCE_LIST is a struct CM_RESOURCE_LIST.
        // REG_FULL_RESOURCE_DESCRIPTOR is a struct CM_FULL_RESOURCE_DESCRIPTOR.
        // REG_RESOURCE_REQUIREMENTS_LIST is a struct IO_RESOURCE_REQUIREMENTS_LIST.
        REG_RESOURCE_LIST | REG_FULL_RESOURCE_DESCRIPTOR | REG_RESOURCE_REQUIREMENTS_LIST => {
            reg_value.vtype = REG_BINARY;
            Value::binary(reg_value.bytes, call_span)
        }
    }
}

fn reg_value_to_nu_string(
    reg_value: winreg::RegValue,
    call_span: Span,
    skip_expand: bool,
) -> nu_protocol::Value {
    let value = String::from_reg_value(&reg_value)
        .expect("registry value type should be REG_SZ or REG_EXPAND_SZ");

    // REG_EXPAND_SZ contains unexpanded references to environment variables, for example, %PATH%.
    // winreg not expanding these is arguably correct, as it's just wrapping raw registry access.
    // These placeholder-having strings work in *some* Windows contexts, but Rust's fs/path APIs
    // don't handle them, so they won't work in Nu unless we expand them here. Eagerly expanding the
    // strings here seems to be the least bad option. This is what PowerShell does, for example,
    // although reg.exe does not. We could do the substitution with our env, but the officially
    // correct way to expand these strings is to call Win32's ExpandEnvironmentStrings function.
    // ref: <https://learn.microsoft.com/en-us/windows/win32/sysinfo/registry-value-types>

    // We can skip the dance if the string doesn't actually have any unexpanded placeholders.
    if skip_expand || reg_value.vtype != REG_EXPAND_SZ || !value.contains('%') {
        return Value::string(value, call_span);
    }

    // The encoding dance is unfortunate since we read "Windows Unicode" from the registry, but
    // it's the most resilient option and avoids making potentially wrong alignment assumptions.
    let value_utf16 = value.encode_utf16().chain([0]).collect::<Vec<u16>>();

    // Like most Win32 string functions, the return value is the number of TCHAR written,
    // or the required buffer size (in TCHAR) if the buffer is too small, or 0 for error.
    // Since we already checked for the case where no expansion is done, we can start with
    // an empty output buffer, since we expect to require at least one resize loop anyway.
    let mut out_buffer = vec![];
    loop {
        match unsafe {
            ExpandEnvironmentStringsW(PCWSTR(value_utf16.as_ptr()), Some(&mut *out_buffer))
        } {
            0 => {
                // 0 means error, but we don't know what the error is. We could try to get
                // the error code with GetLastError, but that's a whole other can of worms.
                // Instead, we'll just return the original string and hope for the best.
                // Presumably, registry strings shouldn't ever cause this to error anyway.
                return Value::string(value, call_span);
            }
            size if size as usize <= out_buffer.len() => {
                // The buffer was large enough, so we're done. Remember to remove the trailing nul!
                let out_value_utf16 = &out_buffer[..size as usize - 1];
                let out_value = String::from_utf16_lossy(out_value_utf16);
                return Value::string(out_value, call_span);
            }
            size => {
                // The buffer was too small, so we need to resize and try again.
                // Clear first to indicate we don't care about the old contents.
                out_buffer.clear();
                out_buffer.resize(size as usize, 0);
                continue;
            }
        }
    }
}

#[test]
fn no_expand_does_not_expand() {
    let unexpanded = "%AppData%";
    let reg_val = || winreg::RegValue {
        bytes: unexpanded
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_ne_bytes)
            .collect(),
        vtype: REG_EXPAND_SZ,
    };

    // normally we do expand
    let nu_val_expanded = reg_value_to_nu_string(reg_val(), Span::unknown(), false);
    assert!(nu_val_expanded.coerce_string().is_ok());
    assert_ne!(nu_val_expanded.coerce_string().unwrap(), unexpanded);

    // unless we skip expansion
    let nu_val_skip_expand = reg_value_to_nu_string(reg_val(), Span::unknown(), true);
    assert!(nu_val_skip_expand.coerce_string().is_ok());
    assert_eq!(nu_val_skip_expand.coerce_string().unwrap(), unexpanded);
}

fn reg_value_to_nu_list_string(reg_value: winreg::RegValue, call_span: Span) -> nu_protocol::Value {
    let values = <Vec<String>>::from_reg_value(&reg_value)
        .expect("registry value type should be REG_MULTI_SZ")
        .into_iter()
        .map(|s| Value::string(s, call_span));

    // There's no REG_MULTI_EXPAND_SZ, so no need to do placeholder expansion here.
    Value::list(values.collect(), call_span)
}

fn reg_value_to_nu_int(reg_value: winreg::RegValue, call_span: Span) -> nu_protocol::Value {
    let value =
        match reg_value.vtype {
            // See discussion here https://github.com/nushell/nushell/pull/10806#issuecomment-1791832088
            // "The unwraps here are effectively infallible...", so I changed them to expects.
            REG_DWORD => u32::from_reg_value(&reg_value)
                .expect("registry value type should be REG_DWORD") as i64,
            REG_DWORD_BIG_ENDIAN => {
                // winreg (v0.51.0) doesn't natively decode REG_DWORD_BIG_ENDIAN
                u32::from_be_bytes(unsafe { *reg_value.bytes.as_ptr().cast() }) as i64
            }
            REG_QWORD => u64::from_reg_value(&reg_value)
                .expect("registry value type should be REG_QWORD") as i64,
            _ => unreachable!(
                "registry value type should be REG_DWORD, REG_DWORD_BIG_ENDIAN, or REG_QWORD"
            ),
        };
    Value::int(value, call_span)
}
