use std::{
    io,
    net::{Ipv4Addr, SocketAddr, TcpListener},
};

pub fn reserve_local_addr() -> io::Result<SocketAddr> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let addr = listener.local_addr()?;
    drop(listener);
    Ok(addr)
}
