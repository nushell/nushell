use std::{
    io,
    net::{Ipv4Addr, SocketAddr, TcpListener},
};

pub fn reserve_local_addr() -> io::Result<SocketAddr> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let addr = listener.local_addr()?;
    // The `TcpListener` impl for WASM does not implement `Drop` for some reason.
    #[cfg_attr(target_arch = "wasm32", expect(clippy::drop_non_drop))]
    drop(listener);
    Ok(addr)
}
