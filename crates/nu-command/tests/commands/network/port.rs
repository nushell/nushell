use nu_test_support::{nu, pipeline};
use std::net::TcpListener;
use std::sync::mpsc;

#[test]
fn port_with_invalid_range() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        port 4000 3999
        "#
    ));

    assert!(actual.err.contains("Invalid range"))
}

#[test]
fn port_with_already_usage() {
    let retry_times = 10;
    for _ in 0..retry_times {
        let (tx, rx) = mpsc::sync_channel(0);

        // let system pick a free port for us.
        let free_port = {
            let listener = TcpListener::bind("127.0.0.1:0").expect("failed to pick a port");
            listener.local_addr().unwrap().port()
        };
        let handler = std::thread::spawn(move || {
            let _listener = TcpListener::bind(format!("127.0.0.1:{free_port}"));
            let _ = rx.recv();
        });
        let actual = nu!(
            cwd: ".", pipeline(&format!("port {free_port} {free_port}"))
        );
        let _ = tx.send(true);
        // make sure that the thread is closed and we release the port.
        handler.join().unwrap();

        // check for error kind str.
        if actual.err.contains("AddrInUse") {
            return;
        }
    }
    panic!("already check port report AddrInUse for seveval times, but still failed.");
}

#[test]
fn port_from_system_given() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        port
        "#
    ));

    // check that we can get an integer port from system.
    assert!(actual.out.parse::<u16>().unwrap() > 0)
}
