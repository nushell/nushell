use nu_protocol::shell_error;
use nu_test_support::prelude::*;
use std::net::{Ipv4Addr, TcpListener};

#[test]
fn port_with_invalid_range() -> Result {
    let err = test().run("port 4000 3999").expect_error()?;
    assert!(matches!(err, ShellError::InvalidRange { .. }));
    Ok(())
}

#[test]
fn port_with_already_usage() -> Result {
    let port = nu_utils::net::reserve_local_addr().unwrap().port();
    let _listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
    let err = test()
        .run(format!("port {port} {port}"))
        .expect_io_error()?;
    assert!(matches!(
        err.kind,
        shell_error::io::ErrorKind::Std(std::io::ErrorKind::AddrInUse, ..),
    ));
    Ok(())
}

#[test]
fn port_from_system_given() -> Result {
    let port: u16 = test().run("port")?;
    // check that we can get an integer port from system.
    assert!(port > 0);
    Ok(())
}

#[test]
fn port_out_of_range() -> Result {
    let err = test().run("port 65536 99999").expect_shell_error()?;
    match err {
        ShellError::CantConvert {
            to_type, from_type, ..
        } => {
            assert_eq!(to_type, "u16");
            assert_eq!(from_type, "usize");
            Ok(())
        }
        err => Err(err.into()),
    }
}
