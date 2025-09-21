use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;

use std::{net::TcpListener, ops::RangeInclusive};

#[derive(Clone)]
pub struct Port;

impl Command for Port {
    fn name(&self) -> &str {
        "port"
    }

    fn signature(&self) -> Signature {
        Signature::build("port")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .optional(
                "start",
                SyntaxShape::Int,
                "The start port to scan (inclusive).",
            )
            .optional("end", SyntaxShape::Int, "The end port to scan (inclusive).")
            .category(Category::Network)
    }

    fn description(&self) -> &str {
        "Get a free TCP port from system."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "http"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        get_free_port(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "get a free port between 3121 and 4000",
                example: "port 3121 4000",
                result: Some(Value::test_int(3121)),
            },
            Example {
                description: "get a free port from system",
                example: "port",
                result: None,
            },
        ]
    }
}

fn get_free_port(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let from_io_error = IoError::factory(call.head, None);

    let start_port: Option<Spanned<usize>> = call.opt(engine_state, stack, 0)?;
    let end_port: Option<Spanned<usize>> = call.opt(engine_state, stack, 1)?;

    let free_port = if start_port.is_none() && end_port.is_none() {
        system_provided_port().map_err(&from_io_error)?
    } else {
        let (start_port, start_span) = match start_port {
            Some(p) => (p.item, Some(p.span)),
            None => (1024, None),
        };

        let start_port = match u16::try_from(start_port) {
            Ok(p) => p,
            Err(e) => {
                return Err(ShellError::CantConvert {
                    to_type: "u16".into(),
                    from_type: "usize".into(),
                    span: start_span.unwrap_or(call.head),
                    help: Some(format!("{e} (min: {}, max: {})", u16::MIN, u16::MAX)),
                });
            }
        };

        let (end_port, end_span) = match end_port {
            Some(p) => (p.item, Some(p.span)),
            None => (65535, None),
        };

        let end_port = match u16::try_from(end_port) {
            Ok(p) => p,
            Err(e) => {
                return Err(ShellError::CantConvert {
                    to_type: "u16".into(),
                    from_type: "usize".into(),
                    span: end_span.unwrap_or(call.head),
                    help: Some(format!("{e} (min: {}, max: {})", u16::MIN, u16::MAX)),
                });
            }
        };

        let range_span = match (start_span, end_span) {
            (Some(start), Some(end)) => Span::new(start.start, end.end),
            (Some(start), None) => start,
            (None, Some(end)) => end,
            (None, None) => call.head,
        };

        // check input range valid.
        if start_port > end_port {
            return Err(ShellError::InvalidRange {
                left_flank: start_port.to_string(),
                right_flank: end_port.to_string(),
                span: range_span,
            });
        }

        search_port_in_range((start_port..=end_port).into_spanned(range_span), call.head)?
    };

    Ok(Value::int(free_port as i64, call.head).into_pipeline_data())
}

fn system_provided_port() -> Result<u16, std::io::Error> {
    TcpListener::bind("127.0.0.1:0")?
        .local_addr()
        .map(|addr| addr.port())
}

/// Find an open port by binding to every possible port in range.
#[cfg(not(windows))]
fn search_port_in_range(
    range: Spanned<RangeInclusive<u16>>,
    call_span: Span,
) -> Result<u16, ShellError> {
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    let listener = 'search: {
        let mut last_err = None;
        for port in range.item {
            let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port));
            match TcpListener::bind(addr) {
                Ok(listener) => break 'search Ok(listener),
                Err(err) => last_err = Some(err),
            }
        }

        Err(IoError::new_with_additional_context(
            last_err.expect("range not empty, validated before"),
            range.span,
            None,
            "Every port has been tried, but no valid one was found",
        ))
    }?;

    Ok(listener
        .local_addr()
        .map_err(|err| IoError::new(err, call_span, None))?
        .port())
}

#[cfg(windows)]
mod windows {
    use super::*;

    use std::{
        alloc::{Layout, alloc, dealloc},
        ptr,
    };

    use ::windows::Win32::{
        Foundation::{
            ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED, NO_ERROR,
            WIN32_ERROR,
        },
        NetworkManagement::IpHelper::{GetTcpTable2, MIB_TCPROW2, MIB_TCPTABLE2},
        Networking::WinSock::ntohs,
    };

    #[repr(C)]
    struct TcpTable {
        pub num_entries: u32,
        pub table: [MIB_TCPROW2],
    }

    const _: () = assert!(align_of::<MIB_TCPTABLE2>() == 4);

    impl TcpTable {
        fn new() -> Result<Box<Self>, WIN32_ERROR> {
            let mut size = 0;
            let size_pointer: *mut u32 = &mut size;

            // SAFETY:
            // - Passing a null table pointer queries the required size (documented behavior).
            // - `size_pointer` is a valid, non-null out pointer.
            // - We expect `ERROR_INSUFFICIENT_BUFFER` so that `size` is written.
            let ret_code = unsafe { GetTcpTable2(None, size_pointer, false) };
            assert_eq!(WIN32_ERROR(ret_code), ERROR_INSUFFICIENT_BUFFER);

            // SAFETY:
            // - Alignment is 4: non-zero and a power of two.
            // - `size` comes from the API and is expected to be reasonable for allocation.
            let layout = unsafe {
                Layout::from_size_align_unchecked(size as usize, align_of::<MIB_TCPTABLE2>())
            };

            // IMPORTANT: This allocation must be freed or transferred to ownership before leaving this scope.
            // SAFETY: `layout` has non-zero size (at least 4 for one u32).
            let ptr = unsafe { alloc(layout) as *mut MIB_TCPTABLE2 };
            assert!(!ptr.is_null());

            // SAFETY:
            // - `ptr` is non-null, properly aligned, and points to `size` bytes.
            // - `size_pointer` still points to `size` from the first call.
            let ret_code = unsafe { GetTcpTable2(Some(ptr), size_pointer, false) };
            let ret_code = WIN32_ERROR(ret_code);
            if ret_code != NO_ERROR {
                // SAFETY:
                // - `ptr` was allocated with `alloc(layout)` in this function.
                // - Using the same `layout` to deallocate is correct.
                unsafe { dealloc(ptr as *mut u8, layout) };
                return Err(ret_code);
            }

            // SAFETY: `GetTcpTable2` returned `NO_ERROR`, so the header at `ptr` is initialized.
            let header = unsafe { &*ptr };

            // SAFETY:
            // - Memory at `ptr` came from the global allocator and is initialized.
            // - `TcpTable` is #[repr(C)] and layout-compatible with `MIB_TCPTABLE2` plus trailing rows.
            // - We build a slice fat pointer only to carry the length; we do not dereference the slice itself here.
            // - Casts between slice DSTs preserve the length metadata:
            //     https://github.com/rust-lang/unsafe-code-guidelines/issues/288
            //     https://github.com/rust-lang/reference/pull/1417
            // - Casting to `*mut TcpTable` preserves that metadata for our DST.
            // - `Box::from_raw` takes ownership and will free via the same allocator.
            let table = unsafe {
                let ptr = ptr::slice_from_raw_parts_mut(ptr, header.dwNumEntries as usize);
                Box::from_raw(ptr as *mut TcpTable)
            };

            Ok(table)
        }
    }

    /// Find an open port by checking the TCP table.
    ///
    /// On Windows, it is possible to bind to the same port multiple times if it was not
    /// originally bound as an exclusive port[^so].
    /// The Rust implementation of [`TcpListener::bind`] currently does not enforce exclusive
    /// binding, which means the same port can be bound more than once.  
    /// Because of this, we cannot simply try binding to a port to check if it is free.  
    /// Instead, we query the [TCP table](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-gettcptable2)
    /// to see which ports are already in use and then pick one that is not listed.
    ///
    /// [^so]: <https://docs.microsoft.com/en-us/windows/win32/winsock/using-so-reuseaddr-and-so-exclusiveaddruse>
    #[cfg(windows)]
    pub fn search_port_in_range(
        range: Spanned<RangeInclusive<u16>>,
        call_span: Span,
    ) -> Result<u16, ShellError> {
        use std::collections::HashSet;

        let table = TcpTable::new()
            .map_err(|err| {
                (
                    err,
                    match err {
                        NO_ERROR => unreachable!("handled as Ok variant"),
                        ERROR_INSUFFICIENT_BUFFER => "The buffer for TcpTable is not large enough",
                        ERROR_INVALID_PARAMETER => "SizePointer was null or not writable",
                        ERROR_NOT_SUPPORTED => "GetTcpTable2 is not supported on this OS",
                        _ => "Unexpected error code from GetTcpTable2",
                    },
                )
            })
            .map_err(|(err, msg)| {
                ShellError::Io(IoError::new_with_additional_context(
                    std::io::Error::from_raw_os_error(err.0 as i32),
                    call_span,
                    None,
                    msg,
                ))
            })?;

        let used_ports: HashSet<u16> = table
            .table
            .iter()
            .map(|row| row.dwLocalPort as u16)
            .map(|raw| {
                // Convert from network byte order to host byte order.
                // SAFETY: `raw` is the exact value returned by the API for a port.
                unsafe { ntohs(raw) }
            })
            .collect();

        for port in range.item {
            if !used_ports.contains(&port) {
                return Ok(port);
            }
        }

        Err(IoError::new_with_additional_context(
            std::io::Error::from(std::io::ErrorKind::AddrInUse),
            call_span,
            None,
            "All ports in the range were taken",
        )
        .into())
    }
}

#[cfg(windows)]
use windows::search_port_in_range;
